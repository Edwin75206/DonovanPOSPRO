use crate::database::Connection;
use crate::{
    db,
    engine::Engine,
    error::{Result, ensure, text},
    models::{Product, ProductAppearance},
    money,
};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{Value, json};
use unicode_normalization::UnicodeNormalization;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryAdjustment {
    pub expected_stock_units: i64,
    #[serde(default)]
    pub reason: String,
}

const PRODUCT_APPEARANCE_JOIN: &str =
    "LEFT JOIN AppMeta a ON a.Key=CONCAT('product-appearance:',p.Id)";

fn valid_appearance(appearance: &ProductAppearance) -> bool {
    [
        "", "cemento", "bloque", "acero", "madera", "pintura", "cable", "electricidad",
        "plomeria", "herramientas", "proteccion", "tornilleria", "medicion", "adhesivos",
        "paquete",
    ]
    .contains(&appearance.icon.as_str())
        && ["", "amarillo", "naranja", "rojo", "verde", "azul", "morado", "agua", "gris"]
            .contains(&appearance.color.as_str())
}

fn read_appearance(record: &mut Value) {
    if let Some(fields) = record.as_object_mut() {
        let appearance = fields
            .remove("appearanceJson")
            .and_then(|value| serde_json::from_str::<ProductAppearance>(value.as_str()?).ok())
            .filter(valid_appearance);
        fields.insert("appearance".into(), json!(appearance));
    }
}

fn product_record(db: &Connection, id: i64) -> Result<Value> {
    let mut record = db::one(
        db,
        &format!(
            "SELECT p.*,a.Value AS AppearanceJson FROM Product p {PRODUCT_APPEARANCE_JOIN} WHERE p.Id=?1"
        ),
        [id],
    )?;
    read_appearance(&mut record);
    Ok(record)
}

pub fn product(db: &Connection, id: i64) -> Result<Product> {
    Ok(serde_json::from_value(db::one(
        db,
        "SELECT * FROM Product WHERE Id=?1",
        [id],
    )?)?)
}
pub fn validate(p: &mut Product) -> Result<()> {
    ensure(
        p.appearance.as_ref().is_none_or(valid_appearance),
        "VALIDATION",
        "Elige un icono y un color disponibles en la biblioteca de materiales.",
    )?;
    p.sku = text(&p.sku, "Clave", 40)?.to_uppercase();
    p.name = text(&p.name, "Descripción", 200)?;
    p.category = text(&p.category, "Categoría", 80)?;
    p.barcode = p
        .barcode
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| text(s, "Código de barras", 64))
        .transpose()?;
    ensure(
        p.brand.len() <= 200 && p.size.len() <= 200 && p.aliases.len() <= 1000,
        "VALIDATION",
        "La marca, medida o términos de búsqueda son demasiado largos.",
    )?;
    ensure(
        (0..=money::MAX_AMOUNT).contains(&p.price_cents)
            && (0..=money::MAX_AMOUNT).contains(&p.wholesale_cents)
            && p.cost_cents
                .is_none_or(|c| (0..=money::MAX_AMOUNT).contains(&c)),
        "VALIDATION",
        "El precio y costo deben ser positivos y estar dentro del límite permitido.",
    )?;
    ensure(
        (0..=money::MAX_UNITS).contains(&p.stock_units)
            && (0..=money::MAX_UNITS).contains(&p.minimum_units)
            && (0..=10000).contains(&p.tax_basis_points),
        "VALIDATION",
        "Revisa las existencias, el mínimo y el impuesto.",
    )?;
    money::quantity(p.unit, p.pack_units)?;
    if p.stock_units > 0 {
        money::quantity(p.unit, p.stock_units)?;
    }
    if p.minimum_units > 0 {
        money::quantity(p.unit, p.minimum_units)?;
    }
    Ok(())
}
pub fn stock(
    db: &Connection,
    now: &str,
    user: i64,
    id: i64,
    delta: i64,
    kind: &str,
    reference: &str,
    reason: &str,
) -> Result<()> {
    let p = product(db, id)?;
    let balance = p
        .stock_units
        .checked_add(delta)
        .filter(|n| *n >= 0 && *n <= money::MAX_UNITS);
    ensure(
        balance.is_some(),
        "INSUFFICIENT_STOCK",
        &format!(
            "No hay suficientes unidades de {}. Disponibles: {:.4}. Modifica la cantidad para continuar.",
            p.name,
            p.stock_units as f64 / 10000.0
        ),
    )?;
    db.execute(
        "UPDATE Product SET StockUnits=?1 WHERE Id=?2",
        params![balance, id],
    )?;
    db.execute("INSERT INTO StockMovement(ProductId,CreatedAt,UserId,Kind,Reference,Reason,DeltaUnits,BalanceUnits) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![id,now,user,kind,reference,reason,delta,balance])?;
    Ok(())
}
fn adjustment_reason(reason: &str) -> Result<String> {
    if reason.trim().is_empty() {
        Ok(String::new())
    } else {
        text(reason, "Motivo", 300)
    }
}
fn record_adjustment(
    db: &Connection,
    now: &str,
    user: i64,
    id: i64,
    delta: i64,
    reason: &str,
) -> Result<()> {
    stock(db, now, user, id, delta, "Ajuste", "Inventario", reason)?;
    db::audit(db, now, Some(user), "Ajustar inventario", &id.to_string(), reason)
}
pub fn persist(db: &Connection, p: &Product) -> Result<i64> {
    let args = params![
        p.sku,
        p.barcode,
        p.name,
        p.category,
        p.unit,
        p.cost_cents,
        p.price_cents,
        p.wholesale_cents,
        p.minimum_units,
        p.tax_basis_points,
        p.pack_name,
        p.pack_units,
        p.active,
        p.brand,
        p.size,
        p.aliases,
        p.id
    ];
    if p.id == 0 {
        db.execute("INSERT INTO Product(Sku,Barcode,Name,Category,Unit,CostCents,PriceCents,WholesaleCents,MinimumUnits,TaxBasisPoints,PackName,PackUnits,Active,Brand,Size,Aliases,StockUnits) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",args)?;
        Ok(db.last_insert_rowid())
    } else {
        let existing = product(db, p.id)?;
        if p.unit != existing.unit {
            ensure(
                existing.stock_units == 0
                    && db::scalar(
                        db,
                        "SELECT (SELECT COUNT(*) FROM StockMovement WHERE ProductId=?1)+(SELECT COUNT(*) FROM QuoteLine WHERE ProductId=?1)+(SELECT COUNT(*) FROM HeldSaleLine WHERE ProductId=?1)",
                        [p.id],
                    )? == 0,
                "UNIT_HAS_HISTORY",
                "Este producto ya tiene existencias o historial. Crea otro producto para utilizar una unidad diferente.",
            )?;
        }
        db.execute("UPDATE Product SET Sku=?1,Barcode=?2,Name=?3,Category=?4,Unit=?5,CostCents=?6,PriceCents=?7,WholesaleCents=?8,MinimumUnits=?9,TaxBasisPoints=?10,PackName=?11,PackUnits=?12,Active=?13,Brand=?14,Size=?15,Aliases=?16 WHERE Id=?17",args)?;
        Ok(p.id)
    }
}
impl Engine {
    pub fn product_for_code(&self, code: &str) -> Result<Value> {
        self.actor("sell")?;
        let code = code.trim();
        ensure(
            !code.is_empty() && code.len() <= 200,
            "INVALID_CODE",
            "Escanea un código o escribe la clave completa.",
        )?;
        let matches = db::all(
            self.db()?,
            "SELECT Id FROM Product WHERE Barcode=?1 OR Sku=?1 COLLATE NOCASE LIMIT 2",
            [code],
        )?;
        ensure(
            !matches.is_empty(),
            "CODE_NOT_FOUND",
            "No encontramos ese código. Busca el nombre y selecciona el producto.",
        )?;
        ensure(
            matches.len() == 1,
            "AMBIGUOUS_CODE",
            "Este código coincide con más de un artículo. Selecciona el producto por nombre y pide al administrador revisar las claves.",
        )?;
        let value = self.get_product(matches[0]["id"].as_i64().unwrap_or(0))?;
        ensure(
            value["active"].as_bool() == Some(true),
            "PRODUCT_UNAVAILABLE",
            "El producto está archivado y no se puede vender.",
        )?;
        ensure(
            value["stockUnits"].as_i64().unwrap_or(0) > 0,
            "INSUFFICIENT_STOCK",
            "Producto agotado. No se agregó al ticket.",
        )?;
        Ok(value)
    }
    pub fn products(&self, search: &str, category: &str, status: &str, page: i64) -> Result<Value> {
        self.product_page(search, category, status, page, "name", false)
    }
    pub fn products_by_brand(
        &self,
        search: &str,
        category: &str,
        brand: &str,
        status: &str,
        page: i64,
    ) -> Result<Value> {
        self.filtered_product_page(search, category, brand, status, page, "name", false)
    }
    pub fn get_product(&self, id: i64) -> Result<Value> {
        let actor = self.actor("sell")?;
        let mut record = product_record(self.db()?, id)?;
        if !actor.can("catalog") {
            record["costCents"] = Value::Null;
        }
        Ok(record)
    }
    pub fn product_page(
        &self,
        search: &str,
        category: &str,
        status: &str,
        page: i64,
        sort: &str,
        descending: bool,
    ) -> Result<Value> {
        self.filtered_product_page(search, category, "", status, page, sort, descending)
    }
    fn filtered_product_page(
        &self,
        search: &str,
        category: &str,
        brand: &str,
        status: &str,
        page: i64,
        sort: &str,
        descending: bool,
    ) -> Result<Value> {
        let actor = self.actor("sell")?;
        ensure(
            search.chars().count() <= 200
                && brand.chars().count() <= 200
                && page >= 0
                && page <= 100000,
            "VALIDATION",
            "Acorta la búsqueda o regresa a la primera página.",
        )?;
        ensure(
            ["active", "archived", "low", "empty"].contains(&status),
            "VALIDATION",
            "Selecciona un filtro válido de existencias.",
        )?;
        let order = match sort {
            "sku" => "p.Sku COLLATE NOCASE",
            "name" => "p.Name COLLATE NOCASE",
            "category" => "p.Category COLLATE NOCASE",
            "stockUnits" => "p.StockUnits",
            "priceCents" => "p.PriceCents",
            "wholesaleCents" => "p.WholesaleCents",
            "costCents" => {
                ensure(
                    actor.can("catalog"),
                    "FORBIDDEN",
                    "Tu cuenta no puede consultar los costos.",
                )?;
                "p.CostCents"
            }
            _ => {
                return Err(crate::error::AppError::rule(
                    "VALIDATION",
                    "Selecciona una columna válida para ordenar.",
                ));
            }
        };
        let direction = if descending { "DESC" } else { "ASC" };
        let normalized: String = search
            .nfkd()
            .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
            .collect();
        let tokens: Vec<String> = normalized
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .take(12)
            .map(|s| format!("\"{s}\"*"))
            .collect();
        let query = tokens.join(" AND ");
        let mut where_sql = "WHERE (?1='' OR p.Id IN (SELECT rowid FROM ProductSearch WHERE ProductSearch MATCH ?1) OR p.Sku=?2 COLLATE NOCASE OR p.Barcode=?2) AND (?3='' OR p.Category=?3) AND ((?4='archived' AND p.Active=0) OR (?4<>'archived' AND p.Active=1)) AND (?4<>'low' OR p.StockUnits<=p.MinimumUnits) AND (?4<>'empty' OR p.StockUnits=0) AND (?5='' OR p.Brand=?5)".to_owned();
        use rusqlite::types::Value as SqlValue;
        let mut bindings = vec![
            SqlValue::Text(query),
            SqlValue::Text(search.trim().into()),
            SqlValue::Text(category.into()),
            SqlValue::Text(status.into()),
            SqlValue::Text(brand.into()),
            SqlValue::Integer(page * 60),
        ];
        if self.db()?.is_mysql() {
            // Includes short hardware measurements and words excluded by default
            // MySQL FULLTEXT stoplists. All tokens remain bound parameters.
            let mut clauses = Vec::new();
            for token in normalized
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .take(12)
            {
                bindings.push(SqlValue::Text(format!("%{token}%")));
                clauses.push(format!("CONCAT_WS(' ',p.Sku,p.Barcode,p.Name,p.Category,p.Brand,p.Size,p.Aliases) COLLATE utf8mb4_0900_ai_ci LIKE ?{}",bindings.len()));
            }
            where_sql = where_sql.replace(
                "p.Id IN (SELECT rowid FROM ProductSearch WHERE ProductSearch MATCH ?1)",
                &format!(
                    "({})",
                    if clauses.is_empty() {
                        "TRUE".into()
                    } else {
                        clauses.join(" AND ")
                    }
                ),
            );
        }
        let count_bindings = if self.db()?.is_mysql() {
            bindings.clone()
        } else {
            bindings[..5].to_vec()
        };
        let total = db::scalar(
            self.db()?,
            &format!("SELECT COUNT(*) FROM Product p {where_sql}"),
            count_bindings,
        )?;
        let mut items = db::all(
            self.db()?,
            &format!(
                "SELECT p.*,a.Value AS AppearanceJson FROM Product p {PRODUCT_APPEARANCE_JOIN} {where_sql} ORDER BY CASE WHEN p.Sku=?2 COLLATE NOCASE OR p.Barcode=?2 THEN 0 ELSE 1 END,{order} {direction},p.Id LIMIT 60 OFFSET ?6"
            ),
            bindings,
        )?;
        let can_view_cost = actor.can("catalog");
        for item in &mut items {
            read_appearance(item);
            if !can_view_cost {
                item["costCents"] = Value::Null;
            }
        }
        let categories = db::all(
            self.db()?,
            "SELECT Category AS name,COUNT(*) AS count FROM Product WHERE Active=1 GROUP BY Category ORDER BY Category",
            [],
        )?;
        let brands = db::all(
            self.db()?,
            "SELECT Brand AS name,COUNT(*) AS count FROM Product WHERE Active=1 AND trim(Brand)<>'' GROUP BY Brand ORDER BY Brand",
            [],
        )?;
        Ok(
            json!({"items":items,"total":total,"page":page,"pageSize":60,"categories":categories,"brands":brands}),
        )
    }
    pub fn save_product(&mut self, p: Product) -> Result<Value> {
        self.save_product_with_stock(p, None)
    }
    pub fn save_product_with_stock(
        &mut self,
        mut p: Product,
        adjustment: Option<InventoryAdjustment>,
    ) -> Result<Value> {
        let actor = self.actor("catalog")?;
        if adjustment.is_some() {
            self.actor("stock")?;
            ensure(p.id > 0, "VALIDATION", "Captura la existencia inicial al dar de alta el producto.")?;
        }
        validate(&mut p)?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let stock_change = if let Some(adjustment) = adjustment {
            let current = product(&tx, p.id)?.stock_units;
            ensure(
                current == adjustment.expected_stock_units,
                "STOCK_CHANGED",
                "Las existencias cambiaron mientras editabas. Vuelve a abrir la ficha para revisar el saldo actual.",
            )?;
            Some((p.stock_units - current, adjustment_reason(&adjustment.reason)?))
        } else {
            None
        };
        let id = persist(&tx, &p)?;
        if let Some(appearance) = &p.appearance {
            let key = format!("product-appearance:{id}");
            if appearance.icon.is_empty() && appearance.color.is_empty() {
                tx.execute("DELETE FROM AppMeta WHERE Key=?1", [key])?;
            } else {
                tx.execute(
                    "INSERT INTO AppMeta(Key,Value) VALUES (?1,?2) ON CONFLICT(Key) DO UPDATE SET Value=excluded.Value",
                    params![key, serde_json::to_string(appearance)?],
                )?;
            }
        }
        if p.id == 0 && p.stock_units > 0 {
            stock(
                &tx,
                &now,
                actor.id,
                id,
                p.stock_units,
                "Alta",
                "Catálogo",
                "Existencia inicial",
            )?;
        }
        if let Some((delta, reason)) = stock_change {
            if delta != 0 {
                record_adjustment(&tx, &now, actor.id, id, delta, &reason)?;
            }
        }
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Guardar producto",
            &id.to_string(),
            "",
        )?;
        let saved = product_record(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn adjust_stock(&mut self, id: i64, delta: i64, reason: &str) -> Result<Value> {
        let actor = self.actor("stock")?;
        let reason = adjustment_reason(reason)?;
        ensure(
            delta != 0,
            "VALIDATION",
            "La cantidad de ajuste no puede ser cero.",
        )?;
        money::quantity(
            product(self.db()?, id)?.unit,
            delta.checked_abs().unwrap_or(i64::MAX),
        )?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        record_adjustment(&tx, &now, actor.id, id, delta, &reason)?;
        let mut result = product_record(&tx, id)?;
        if !actor.can("catalog") {
            result["costCents"] = Value::Null;
        }
        tx.commit()?;
        Ok(result)
    }
    pub fn stock_history(&self, id: i64) -> Result<Value> {
        self.actor("sell")?;
        Ok(json!(db::all(
            self.db()?,
            "SELECT * FROM StockMovement WHERE ProductId=?1 ORDER BY Id DESC LIMIT 500",
            [id]
        )?))
    }
}
