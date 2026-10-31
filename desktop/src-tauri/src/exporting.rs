use crate::{
    Engine, db,
    error::{AppError, Result, ensure},
    importing::FIELDS,
    models::Product,
    storage,
};
use rust_xlsxwriter::{Color, Format, Image, Workbook};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
fn excel(error: impl std::fmt::Display) -> AppError {
    AppError::internal(
        "EXPORT_FAILED",
        "No pudimos preparar el archivo de Excel. Revisa el espacio disponible e intenta nuevamente.",
        error,
    )
}
fn save(workbook: &mut Workbook, folder: &Path, name: &str) -> Result<Value> {
    ensure(
        folder.is_dir(),
        "INVALID_PATH",
        "Selecciona una carpeta existente para guardar el archivo.",
    )?;
    let folder = folder.canonicalize()?;
    let bytes = workbook.save_to_buffer().map_err(excel)?;
    storage::require_space(&folder, bytes.len() as u64)?;
    let filename = format!("{name}-{}.xlsx", &uuid::Uuid::new_v4().to_string()[..8]);
    let destination = folder.join(filename);
    let partial = destination.with_extension("partial");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        storage::move_new(&partial, &destination)?;
        Ok(json!({"path":storage::display_path(&destination),"bytes":bytes.len()}))
    })();
    if partial.exists() {
        let _ = fs::remove_file(partial);
    }
    result
}
impl Engine {
    pub fn export_products(&self, folder: &Path, template: bool) -> Result<Value> {
        self.actor("catalog")?;
        let mut workbook = Workbook::new();
        let headers = Format::new()
            .set_bold()
            .set_font_color(Color::White)
            .set_background_color(Color::RGB(0xCF2928))
            .set_text_wrap();
        let money = Format::new().set_num_format("#,##0.00");
        let quantity = Format::new().set_num_format("0.####");
        let text = Format::new().set_num_format("@");
        let sheet = workbook.add_worksheet();
        sheet.set_name("Productos").map_err(excel)?;
        sheet.set_freeze_panes(1, 3).map_err(excel)?;
        sheet.set_row_height(0, 32).map_err(excel)?;
        for (i, (_, label)) in FIELDS.iter().enumerate() {
            sheet
                .write_string_with_format(0, i as u16, *label, &headers)
                .map_err(excel)?;
            sheet
                .set_column_width(
                    i as u16,
                    if i == 2 {
                        42.0
                    } else if i == 15 {
                        30.0
                    } else {
                        19.0
                    },
                )
                .map_err(excel)?;
        }
        for c in [0, 1] {
            sheet.set_column_format(c, &text).map_err(excel)?;
        }
        for c in [7, 8, 9] {
            sheet.set_column_format(c, &money).map_err(excel)?;
        }
        for c in [10, 11, 12, 14] {
            sheet.set_column_format(c, &quantity).map_err(excel)?;
        }
        if !template {
            let products: Vec<Product> = db::all(
                self.db()?,
                "SELECT * FROM Product ORDER BY Sku COLLATE NOCASE",
                [],
            )?
            .into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<_, _>>()?;
            ensure(
                products.len() <= 50000,
                "EXPORT_LIMIT",
                "Este catálogo supera 50,000 productos. Exporta un reporte filtrado para trabajar con archivos más pequeños.",
            )?;
            for (i, p) in products.iter().enumerate() {
                let row = i as u32 + 1;
                for (col, value) in [
                    (0, p.sku.as_str()),
                    (1, p.barcode.as_deref().unwrap_or("")),
                    (2, &p.name),
                    (3, &p.category),
                    (4, &p.brand),
                    (5, &p.size),
                    (
                        6,
                        ["Pieza", "Metro", "Kilogramo", "Litro", "Caja", "Rollo"]
                            .get(p.unit as usize)
                            .copied()
                            .unwrap_or("Pieza"),
                    ),
                    (13, &p.pack_name),
                    (15, &p.aliases),
                    (16, if p.active { "Sí" } else { "No" }),
                ] {
                    sheet.write_string(row, col, value).map_err(excel)?;
                }
                if let Some(cost) = p.cost_cents {
                    sheet
                        .write_number(row, 7, cost as f64 / 100.0)
                        .map_err(excel)?;
                }
                for (col, value) in [
                    (8, p.price_cents as f64 / 100.0),
                    (9, p.wholesale_cents as f64 / 100.0),
                    (10, p.stock_units as f64 / 10000.0),
                    (11, p.minimum_units as f64 / 10000.0),
                    (12, p.tax_basis_points as f64 / 100.0),
                    (14, p.pack_units as f64 / 10000.0),
                ] {
                    sheet.write_number(row, col, value).map_err(excel)?;
                }
            }
            sheet
                .autofilter(0, 0, products.len() as u32, 16)
                .map_err(excel)?;
        }
        let settings = db::one(
            self.db()?,
            "SELECT StoreName,LogoData FROM StoreSettings LIMIT 1",
            [],
        )?;
        let instructions = workbook.add_worksheet();
        instructions.set_name("Instrucciones").map_err(excel)?;
        instructions.set_column_width(0, 38).map_err(excel)?;
        instructions.set_column_width(1, 95).map_err(excel)?;
        instructions
            .write_string(0, 0, settings["storeName"].as_str().unwrap_or("Mi negocio"))
            .map_err(excel)?;
        let wrapping = Format::new().set_text_wrap();
        for (i,(label,value)) in [("Uso","Completa la hoja Productos. Los encabezados deben estar en la fila 1. Puedes asignar columnas distintas desde la vista previa de importación."),("Claves y códigos","Usa formato Texto, especialmente en códigos de barras, para conservar ceros iniciales y todos los dígitos."),("Precios y costo","El precio y mayoreo incluyen IVA; el costo es neto. Máximo 2 decimales. Usa punto decimal y coma solo para miles."),("Cantidades","Metro, Kilogramo y Litro admiten hasta 4 decimales. Pieza, Caja y Rollo requieren cantidades enteras."),("Fórmulas","Pega los resultados como valores. La importación no ejecuta fórmulas, macros ni conexiones externas."),("Actualizaciones","Por defecto se conservan los precios, costos y existencias de productos existentes. Cualquier sustitución se elige y confirma en la vista previa."),("Límites","Libro .xlsx de hasta 20 MB; 50,000 productos y 64 columnas por hoja. Activo acepta Sí o No."),("Respaldo","Un archivo de productos no sustituye el respaldo completo del negocio. Usa Administración de datos para conservar ventas, usuarios y demás información.")].iter().enumerate(){instructions.write_string((i+2)as u32,0,*label).map_err(excel)?;instructions.write_string_with_format((i+2)as u32,1,*value,&wrapping).map_err(excel)?;instructions.set_row_height((i+2)as u32,44).map_err(excel)?;}
        if let Some(encoded) = settings["logoData"].as_str() {
            use base64::Engine as _;
            if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(image) = Image::new_from_buffer(&data) {
                    instructions
                        .insert_image(12, 0, &image.set_scale_to_size(240, 160, true))
                        .map_err(excel)?;
                }
            }
        }
        save(
            &mut workbook,
            folder,
            if template {
                "Plantilla-productos"
            } else {
                "Catalogo-productos"
            },
        )
    }
}
