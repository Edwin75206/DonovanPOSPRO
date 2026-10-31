use crate::{
    Engine, catalog, db,
    error::{AppError, Result, ensure},
    models::Product,
    operations,
};
use calamine::{DataRef, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufReader, Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use unicode_normalization::UnicodeNormalization;

pub const FIELDS: [(&str, &str); 17] = [
    ("sku", "Clave"),
    ("barcode", "Código de barras"),
    ("name", "Descripción"),
    ("category", "Categoría"),
    ("brand", "Marca"),
    ("size", "Medidas"),
    ("unit", "Unidad"),
    ("cost", "Costo neto"),
    ("price", "Precio con IVA"),
    ("wholesale", "Mayoreo con IVA"),
    ("stock", "Existencia"),
    ("minimum", "Mínimo"),
    ("tax", "IVA %"),
    ("packName", "Presentación"),
    ("packUnits", "Unidades por presentación"),
    ("aliases", "Términos de búsqueda"),
    ("active", "Activo"),
];
#[derive(Clone, Default)]
struct Cell {
    text: String,
    numeric: bool,
    formula: bool,
    unsupported: bool,
}
struct Workbook {
    bytes: Arc<[u8]>,
    file_name: String,
    owner: i64,
    origin: PathBuf,
    sheets: Vec<String>,
    sheet: String,
    headers: Vec<String>,
    rows: Vec<(u32, Vec<Cell>)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportOptions {
    pub mapping: BTreeMap<String, usize>,
    pub mode: String,
    pub prices: String,
    pub stock: String,
}
#[derive(Clone)]
struct Change {
    row: u32,
    product: Product,
    before: Option<Product>,
    fingerprint: Option<String>,
    stock_delta: i64,
}
struct Plan {
    id: String,
    owner: i64,
    origin: PathBuf,
    created_at: i64,
    changes: Vec<Change>,
    summary: Value,
}
#[derive(Default)]
pub struct ImportState {
    workbook: Option<Workbook>,
    plan: Option<Plan>,
}
fn invalid(error: impl std::fmt::Display) -> AppError {
    AppError::internal(
        "INVALID_EXCEL",
        "No pudimos leer el libro. Guarda una copia en formato .xlsx y vuelve a seleccionarla.",
        error,
    )
}
fn normalized(s: &str) -> String {
    s.nfkd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c) && c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn cell_text(value: &DataRef<'_>) -> String {
    match value {
        DataRef::Empty => String::new(),
        DataRef::String(s) => s.clone(),
        DataRef::SharedString(s) => (*s).to_owned(),
        DataRef::Int(n) => n.to_string(),
        DataRef::Float(n) => n.to_string(),
        DataRef::Bool(b) => if *b { "Sí" } else { "No" }.into(),
        _ => "Fecha o error de Excel".into(),
    }
}

// Bound compressed content before parsing. Immutable bytes and streamed cells prevent a
// changed source file or an oversized sparse worksheet from allocating an unbounded range.
fn preflight(bytes: &[u8]) -> Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(invalid)?;
    ensure(
        archive.len() <= 1000,
        "INVALID_EXCEL",
        "El libro contiene demasiados archivos. Copia la hoja de productos a un libro nuevo.",
    )?;
    let mut total = 0u64;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(invalid)?;
        total = total.saturating_add(entry.size());
        ensure(
            entry.size() <= 80 * 1024 * 1024 && total <= 120 * 1024 * 1024,
            "INVALID_EXCEL",
            "El contenido del libro supera el límite de esta importación.",
        )?;
        if entry.name().starts_with("xl/worksheets/") && entry.name().ends_with(".xml") {
            let mut reader =
                quick_xml::Reader::from_reader(BufReader::new(entry.take(80 * 1024 * 1024 + 1)));
            let mut buffer = vec![];
            loop {
                match reader.read_event_into(&mut buffer).map_err(invalid)? {
                    quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e)
                        if e.local_name().as_ref() == b"c" =>
                    {
                        for attr in e.attributes() {
                            let attr = attr.map_err(invalid)?;
                            if attr.key.as_ref() == b"r" {
                                let reference =
                                    std::str::from_utf8(&attr.value).map_err(invalid)?;
                                let mut column = 0u32;
                                let mut row = 0u32;
                                let mut digits = false;
                                for b in reference.bytes() {
                                    if b.is_ascii_uppercase() && !digits {
                                        column = column
                                            .saturating_mul(26)
                                            .saturating_add((b - b'A' + 1) as u32);
                                    } else if b.is_ascii_digit() {
                                        digits = true;
                                        row = row
                                            .saturating_mul(10)
                                            .saturating_add((b - b'0') as u32);
                                    } else {
                                        return Err(AppError::rule(
                                            "INVALID_EXCEL",
                                            "El libro contiene referencias de celdas no válidas.",
                                        ));
                                    }
                                }
                                ensure(
                                    column > 0 && column <= 64 && row > 0 && row <= 50001,
                                    "EXCEL_LIMIT",
                                    "La hoja admite hasta 50,000 productos y 64 columnas, con encabezados en la fila 1. Copia solo ese rango a un libro nuevo.",
                                )?;
                            }
                        }
                    }
                    quick_xml::events::Event::Eof => break,
                    _ => {}
                }
                buffer.clear();
            }
        }
    }
    Ok(())
}
fn worksheet(book: &mut Workbook, sheet: &str) -> Result<Value> {
    ensure(
        book.sheets.iter().any(|s| s == sheet),
        "VALIDATION",
        "Selecciona una hoja válida.",
    )?;
    let mut reader = Xlsx::new(Cursor::new(book.bytes.clone())).map_err(invalid)?;
    let mut rows: BTreeMap<u32, Vec<Cell>> = BTreeMap::new();
    let mut width = 0usize;
    let mut total = 0usize;
    {
        let mut stream = reader.worksheet_cells_reader(sheet).map_err(invalid)?;
        while let Some(cell) = stream.next_cell().map_err(invalid)? {
            let (row, col) = cell.get_position();
            ensure(
                row <= 50000 && col < 64,
                "EXCEL_LIMIT",
                "La hoja admite 50,000 productos y 64 columnas.",
            )?;
            let value = cell.get_value();
            let text = cell_text(value);
            ensure(
                text.len() <= 4000,
                "EXCEL_LIMIT",
                &format!("La fila {} contiene una celda demasiado larga.", row + 1),
            )?;
            total = total.saturating_add(text.len());
            ensure(
                total <= 64 * 1024 * 1024,
                "EXCEL_LIMIT",
                "El texto de la hoja supera el tamaño admitido. Divide el archivo en varias importaciones.",
            )?;
            if text.is_empty() {
                continue;
            }
            width = width.max(col as usize + 1);
            let values = rows.entry(row).or_default();
            values.resize_with(values.len().max(col as usize + 1), Cell::default);
            values[col as usize] = Cell {
                text,
                numeric: matches!(value, DataRef::Int(_) | DataRef::Float(_)),
                formula: false,
                unsupported: matches!(
                    value,
                    DataRef::DateTime(_)
                        | DataRef::DateTimeIso(_)
                        | DataRef::DurationIso(_)
                        | DataRef::Error(_)
                ),
            };
        }
    }
    {
        let mut stream = reader.worksheet_cells_reader(sheet).map_err(invalid)?;
        while let Some(cell) = stream.next_formula().map_err(invalid)? {
            let (row, col) = cell.get_position();
            ensure(
                row <= 50000 && col < 64,
                "EXCEL_LIMIT",
                "La hoja admite 50,000 productos y 64 columnas.",
            )?;
            if !cell.get_value().is_empty() {
                width = width.max(col as usize + 1);
                let values = rows.entry(row).or_default();
                values.resize_with(values.len().max(col as usize + 1), Cell::default);
                values[col as usize].formula = true;
            }
        }
    }
    let header = rows
        .remove(&0)
        .ok_or_else(|| AppError::rule("INVALID_EXCEL", "Coloca los encabezados en la fila 1."))?;
    book.headers = (0..width)
        .map(|c| {
            header
                .get(c)
                .map(|c| c.text.clone())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Columna {}", c + 1))
        })
        .collect();
    book.rows = rows
        .into_iter()
        .map(|(row, mut cells)| {
            cells.resize_with(width, Cell::default);
            (row + 1, cells)
        })
        .collect();
    book.sheet = sheet.to_owned();
    let mut mapping = BTreeMap::new();
    for (key, label) in FIELDS {
        if let Some(index) = book
            .headers
            .iter()
            .position(|h| normalized(h) == normalized(label))
        {
            mapping.insert(key, index);
        }
    }
    Ok(
        json!({"fileName":book.file_name,"sheets":book.sheets,"sheet":book.sheet,"headers":book.headers,"rows":book.rows.len(),"sample":book.rows.iter().take(5).map(|(r,c)|json!({"row":r,"values":c.iter().map(|c|c.text.clone()).collect::<Vec<_>>()})).collect::<Vec<_>>(),"mapping":mapping,"fields":FIELDS.iter().map(|(key,label)|json!({"key":key,"label":label})).collect::<Vec<_>>()}),
    )
}
fn decimal(s: &str, scale: u32, label: &str) -> Result<i64> {
    let value = s.trim().trim_start_matches('$').trim();
    ensure(
        !value.is_empty(),
        "INVALID_ROW",
        &format!("{label}: falta un valor numérico."),
    )?;
    let pieces = value.split('.').collect::<Vec<_>>();
    ensure(
        pieces.len() <= 2,
        "INVALID_ROW",
        &format!("{label}: usa punto decimal y coma únicamente para miles."),
    )?;
    let groups = pieces[0].split(',').collect::<Vec<_>>();
    ensure(
        groups.len() == 1
            || (groups[0].len() <= 3
                && !groups[0].is_empty()
                && groups.iter().skip(1).all(|g| g.len() == 3)),
        "INVALID_ROW",
        &format!("{label}: revisa la separación de miles."),
    )?;
    let whole = groups.join("");
    let fraction = pieces.get(1).copied().unwrap_or("");
    ensure(
        !whole.is_empty()
            && whole.bytes().all(|b| b.is_ascii_digit())
            && fraction.bytes().all(|b| b.is_ascii_digit())
            && fraction.len() <= scale as usize,
        "INVALID_ROW",
        &format!("{label}: utiliza un valor positivo con hasta {scale} decimales."),
    )?;
    let n = whole
        .parse::<i128>()
        .ok()
        .and_then(|n| n.checked_mul(10i128.pow(scale)))
        .and_then(|n| {
            fraction
                .parse::<i128>()
                .or(if fraction.is_empty() { Ok(0) } else { Err(()) })
                .ok()
                .and_then(|f| n.checked_add(f * 10i128.pow(scale - fraction.len() as u32)))
        });
    n.and_then(|n| i64::try_from(n).ok()).ok_or_else(|| {
        AppError::rule(
            "INVALID_ROW",
            &format!("{label}: el valor es demasiado grande."),
        )
    })
}
fn mapped<'a>(cells: &'a [Cell], options: &ImportOptions, key: &str) -> Result<Option<&'a Cell>> {
    let cell = options.mapping.get(key).and_then(|i| cells.get(*i));
    let label = FIELDS
        .iter()
        .find(|f| f.0 == key)
        .map(|f| f.1)
        .unwrap_or(key);
    if cell.is_some_and(|c| c.formula) {
        return Err(AppError::rule(
            "INVALID_ROW",
            &format!(
                "{label} contiene una fórmula. Pega su resultado como valor antes de importar."
            ),
        ));
    }
    ensure(
        cell.is_none_or(|c| !c.unsupported),
        "INVALID_ROW",
        &format!(
            "{label}: utiliza texto o números; la celda tiene formato de fecha o un error de Excel."
        ),
    )?;
    Ok(cell)
}
fn apply(cells: &[Cell], options: &ImportOptions, mut p: Product) -> Result<(Product, i64)> {
    let new = p.id == 0;
    let old_stock = p.stock_units;
    for (key, target) in [
        ("sku", &mut p.sku),
        ("name", &mut p.name),
        ("category", &mut p.category),
        ("brand", &mut p.brand),
        ("size", &mut p.size),
        ("packName", &mut p.pack_name),
        ("aliases", &mut p.aliases),
    ] {
        if let Some(c) = mapped(cells, options, key)? {
            *target = c.text.trim().to_owned();
        }
    }
    if let Some(c) = mapped(cells, options, "barcode")? {
        ensure(
            !c.numeric,
            "INVALID_ROW",
            "Código de barras: usa celdas de texto para conservar todos los dígitos y ceros iniciales.",
        )?;
        p.barcode = (!c.text.trim().is_empty()).then(|| c.text.trim().to_owned());
    }
    if let Some(c) = mapped(cells, options, "unit")? {
        p.unit = match normalized(&c.text).as_str() {
            "pieza" | "pza" => 0,
            "metro" | "m" => 1,
            "kilogramo" | "kg" => 2,
            "litro" | "l" => 3,
            "caja" => 4,
            "rollo" => 5,
            _ => {
                return Err(AppError::rule(
                    "INVALID_ROW",
                    "Unidad: usa Pieza, Metro, Kilogramo, Litro, Caja o Rollo.",
                ));
            }
        };
    }
    if new || options.prices == "replace" {
        if new {
            ensure(
                options.mapping.contains_key("price"),
                "INVALID_ROW",
                "Asigna la columna Precio con IVA para crear productos.",
            )?;
        }
        for (key, target) in [
            ("price", &mut p.price_cents),
            ("wholesale", &mut p.wholesale_cents),
        ] {
            if let Some(c) = mapped(cells, options, key)? {
                *target = if c.text.trim().is_empty() && key == "wholesale" {
                    0
                } else {
                    decimal(&c.text, 2, key)?
                };
            }
        }
        if let Some(c) = mapped(cells, options, "cost")? {
            p.cost_cents = if c.text.trim().is_empty() {
                None
            } else {
                Some(decimal(&c.text, 2, "Costo neto")?)
            };
        }
    }
    for (key, target, scale) in [
        ("minimum", &mut p.minimum_units, 4),
        ("packUnits", &mut p.pack_units, 4),
        ("tax", &mut p.tax_basis_points, 2),
    ] {
        if let Some(c) = mapped(cells, options, key)? {
            if !c.text.trim().is_empty() {
                *target = decimal(&c.text, scale, key)?;
            }
        }
    }
    if let Some(c) = mapped(cells, options, "active")? {
        p.active = match normalized(&c.text).as_str() {
            "si" | "true" | "1" | "activo" => true,
            "no" | "false" | "0" | "inactivo" => false,
            _ => return Err(AppError::rule("INVALID_ROW", "Activo: escribe Sí o No.")),
        };
    }
    if new || options.stock != "keep" {
        if let Some(c) = mapped(cells, options, "stock")? {
            let amount = if c.text.trim().is_empty() {
                0
            } else {
                decimal(&c.text, 4, "Existencia")?
            };
            p.stock_units = if !new && options.stock == "add" {
                old_stock.checked_add(amount).ok_or_else(|| {
                    AppError::rule("INVALID_ROW", "La existencia supera el límite permitido.")
                })?
            } else {
                amount
            };
        }
    }
    catalog::validate(&mut p)?;
    let delta = p.stock_units - old_stock;
    Ok((p, delta))
}
impl Engine {
    pub fn open_import(&mut self, path: &Path) -> Result<Value> {
        let actor = self.actor("catalog")?;
        self.importing.plan = None;
        let mut bytes = vec![];
        File::open(path)?
            .take(20 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure(
            bytes.len() <= 20 * 1024 * 1024,
            "EXCEL_LIMIT",
            "Selecciona un libro .xlsx de hasta 20 MB.",
        )?;
        preflight(&bytes)?;
        let bytes: Arc<[u8]> = bytes.into();
        let reader = Xlsx::new(Cursor::new(bytes.clone())).map_err(invalid)?;
        let sheets = reader.sheet_names().clone();
        ensure(
            !sheets.is_empty() && sheets.len() <= 32,
            "EXCEL_LIMIT",
            "El libro debe contener entre 1 y 32 hojas.",
        )?;
        let mut book = Workbook {
            bytes,
            file_name: path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Productos.xlsx")
                .into(),
            owner: actor.id,
            origin: self.db_path.clone(),
            sheet: String::new(),
            sheets: sheets.clone(),
            headers: vec![],
            rows: vec![],
        };
        let preview = worksheet(&mut book, &sheets[0])?;
        self.importing = ImportState {
            workbook: Some(book),
            plan: None,
        };
        Ok(preview)
    }
    pub fn import_sheet(&mut self, sheet: &str) -> Result<Value> {
        let actor = self.actor("catalog")?;
        let book =
            self.importing.workbook.as_mut().ok_or_else(|| {
                AppError::rule("PLAN_EXPIRED", "Selecciona nuevamente el archivo.")
            })?;
        ensure(
            book.owner == actor.id && book.origin == self.db_path,
            "PLAN_CHANGED",
            "La sesión o la base cambió. Selecciona nuevamente el archivo.",
        )?;
        self.importing.plan = None;
        worksheet(book, sheet)
    }
    pub fn validate_import(&mut self, options: ImportOptions) -> Result<Value> {
        let actor = self.actor("catalog")?;
        ensure(
            ["add", "update", "upsert"].contains(&options.mode.as_str())
                && ["keep", "replace"].contains(&options.prices.as_str())
                && ["keep", "replace", "add"].contains(&options.stock.as_str()),
            "VALIDATION",
            "Selecciona cómo tratar los productos, precios y existencias.",
        )?;
        let book =
            self.importing.workbook.as_ref().ok_or_else(|| {
                AppError::rule("PLAN_EXPIRED", "Selecciona nuevamente el archivo.")
            })?;
        ensure(
            book.owner == actor.id && book.origin == self.db_path,
            "PLAN_CHANGED",
            "La sesión o base cambió. Selecciona nuevamente el archivo.",
        )?;
        ensure(
            options.mapping.contains_key("sku")
                && options
                    .mapping
                    .iter()
                    .all(|(k, i)| FIELDS.iter().any(|f| f.0 == k) && *i < book.headers.len()),
            "VALIDATION",
            "Asigna la columna Clave y revisa las columnas seleccionadas.",
        )?;
        ensure(
            !book.rows.is_empty(),
            "EMPTY_FILE",
            "No hay productos en esta hoja. Añade filas debajo de los encabezados.",
        )?;
        let mut unique = HashSet::new();
        ensure(
            options.mapping.values().all(|i| unique.insert(*i)),
            "VALIDATION",
            "Cada columna del archivo debe asignarse a un solo campo.",
        )?;
        let mut skus = HashSet::new();
        let mut codes = HashSet::new();
        let mut changes = vec![];
        let mut errors = vec![];
        let mut failed = 0;
        let mut preview = vec![];
        let mut price_changes = 0;
        let mut stock_changes = 0;
        let mut added = 0;
        let default_tax = db::scalar(
            self.db()?,
            "SELECT DefaultTaxBasisPoints FROM StoreSettings LIMIT 1",
            [],
        )?;
        for (row, cells) in &book.rows {
            let result: Result<(Product, i64, Option<Product>)> = (|| {
                let sku = mapped(cells, &options, "sku")?
                    .map(|c| c.text.trim().to_uppercase())
                    .unwrap_or_default();
                ensure(
                    skus.insert(sku.clone()),
                    "INVALID_ROW",
                    "La clave está duplicada dentro del archivo.",
                )?;
                let before = db::all(
                    self.db()?,
                    "SELECT * FROM Product WHERE Sku=?1 COLLATE NOCASE",
                    [&sku],
                )?
                .into_iter()
                .next()
                .map(serde_json::from_value::<Product>)
                .transpose()?;
                ensure(
                    !(before.is_some() && options.mode == "add"),
                    "INVALID_ROW",
                    "La clave ya existe. Selecciona actualizar o agregar y actualizar.",
                )?;
                ensure(
                    !(before.is_none() && options.mode == "update"),
                    "INVALID_ROW",
                    "La clave no existe. Selecciona agregar y actualizar para crearla.",
                )?;
                let (p, delta) = apply(
                    cells,
                    &options,
                    before.clone().unwrap_or_else(|| Product {
                        tax_basis_points: default_tax,
                        ..Default::default()
                    }),
                )?;
                if let Some(old) = &before {
                    if p.unit != old.unit {
                        ensure(
                            old.stock_units == 0
                                && db::scalar(
                                    self.db()?,
                                    "SELECT (SELECT COUNT(*) FROM StockMovement WHERE ProductId=?1)+(SELECT COUNT(*) FROM QuoteLine WHERE ProductId=?1)+(SELECT COUNT(*) FROM HeldSaleLine WHERE ProductId=?1)",
                                    [p.id],
                                )? == 0,
                            "UNIT_HAS_HISTORY",
                            "El producto tiene historial; no se puede cambiar su unidad.",
                        )?;
                    }
                }
                if let Some(code) = &p.barcode {
                    ensure(
                        codes.insert(code.clone()),
                        "INVALID_ROW",
                        "El código de barras se repite dentro del archivo.",
                    )?;
                    ensure(
                        db::scalar(
                            self.db()?,
                            "SELECT COUNT(*) FROM Product WHERE Barcode=?1 AND Id<>?2",
                            rusqlite::params![code, p.id],
                        )? == 0,
                        "INVALID_ROW",
                        "El código de barras pertenece a otro producto.",
                    )?;
                }
                Ok((p, delta, before))
            })();
            match result {
                Ok((p, delta, before)) => {
                    let price_changed = before.as_ref().is_some_and(|b| {
                        b.price_cents != p.price_cents
                            || b.wholesale_cents != p.wholesale_cents
                            || b.cost_cents != p.cost_cents
                    });
                    price_changes += usize::from(price_changed);
                    stock_changes += usize::from(delta != 0);
                    added += usize::from(before.is_none());
                    if preview.len() < 200 {
                        preview.push(json!({"id":row,"row":row,"sku":p.sku,"name":p.name,"action":if before.is_none(){"Agregar"}else{"Actualizar"},"oldPriceCents":before.as_ref().map(|b|b.price_cents),"priceCents":p.price_cents,"oldStockUnits":before.as_ref().map(|b|b.stock_units),"stockUnits":p.stock_units,"unit":p.unit}));
                    }
                    let fingerprint = before.as_ref().map(operations::fingerprint).transpose()?;
                    changes.push(Change {
                        row: *row,
                        product: p,
                        before,
                        fingerprint,
                        stock_delta: delta,
                    });
                }
                Err(error) => {
                    failed += 1;
                    if errors.len() < 200 {
                        errors.push(json!({"id":row,"row":row,"message":error.message}));
                    }
                }
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let result = json!({"planId":id,"valid":failed==0,"total":book.rows.len(),"added":added,"updated":changes.len()-added,"priceChanges":price_changes,"stockChanges":stock_changes,"errorCount":failed,"errors":errors,"rows":preview});
        self.importing.plan = if failed == 0 {
            Some(Plan {
                id,
                owner: actor.id,
                origin: self.db_path.clone(),
                created_at: self.now(),
                changes,
                summary: result.clone(),
            })
        } else {
            None
        };
        Ok(result)
    }
    pub fn commit_import(&mut self, id: &str) -> Result<Value> {
        let actor = self.actor("catalog")?;
        if let Some(result) = operations::previous(self.db()?, &actor, id, "import", id)? {
            return Ok(result);
        }
        let plan = self.importing.plan.as_ref().ok_or_else(|| {
            AppError::rule(
                "PLAN_EXPIRED",
                "Valida nuevamente el archivo antes de importar.",
            )
        })?;
        ensure(
            plan.id == id
                && plan.owner == actor.id
                && plan.origin == self.db_path
                && self.now() >= plan.created_at
                && self.now() - plan.created_at <= 900,
            "PLAN_EXPIRED",
            "La vista previa venció o cambió la sesión. Valida nuevamente el archivo.",
        )?;
        let changes = plan.changes.clone();
        let summary = plan.summary.clone();
        let now = self.now_string();
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for change in changes {
            let p = &change.product;
            if change.before.is_some() {
                let current = catalog::product(&tx, p.id)?;
                ensure(
                    Some(operations::fingerprint(&current)?) == change.fingerprint,
                    "CATALOG_CHANGED",
                    &format!(
                        "Fila {}: {} cambió desde la vista previa. Vuelve a validar el archivo; no se importó ninguna fila.",
                        change.row, p.sku
                    ),
                )?;
            } else {
                ensure(
                    db::scalar(
                        &tx,
                        "SELECT COUNT(*) FROM Product WHERE Sku=?1 COLLATE NOCASE",
                        [&p.sku],
                    )? == 0,
                    "CATALOG_CHANGED",
                    "Una clave se agregó después de validar el archivo. Vuelve a revisar la vista previa.",
                )?;
            }
            let saved = catalog::persist(&tx, p)?;
            if change.stock_delta != 0 {
                catalog::stock(
                    &tx,
                    &now,
                    actor.id,
                    saved,
                    change.stock_delta,
                    "Importación",
                    id,
                    "Existencia autorizada en la vista previa de Excel",
                )?;
            }
        }
        let result = json!({"added":summary["added"],"updated":summary["updated"],"priceChanges":summary["priceChanges"],"stockChanges":summary["stockChanges"]});
        operations::receipt(&tx, &actor, id, "import", id, &result, &now)?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Importar productos",
            id,
            &format!(
                "{} altas; {} actualizaciones",
                summary["added"], summary["updated"]
            ),
        )?;
        tx.commit()?;
        self.importing = ImportState::default();
        Ok(result)
    }
}
