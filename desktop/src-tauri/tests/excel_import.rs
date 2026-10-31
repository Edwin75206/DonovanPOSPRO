mod common;
use common::*;
use donovan_core::{catalog, db, importing::ImportOptions, models::Product};
use rust_xlsxwriter::{Formula, Workbook};
use std::{collections::BTreeMap, path::Path};
fn options() -> ImportOptions {
    ImportOptions {
        mapping: BTreeMap::from([
            ("sku".into(), 0),
            ("name".into(), 1),
            ("price".into(), 2),
            ("stock".into(), 3),
            ("unit".into(), 4),
            ("barcode".into(), 5),
        ]),
        mode: "upsert".into(),
        prices: "keep".into(),
        stock: "keep".into(),
    }
}
fn workbook(path: &Path, rows: &[(&str, &str, &str, &str, &str, &str)]) {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    for (i, h) in [
        "Clave",
        "Descripción",
        "Precio con IVA",
        "Existencia",
        "Unidad",
        "Código de barras",
    ]
    .iter()
    .enumerate()
    {
        sheet.write_string(0, i as u16, *h).unwrap();
    }
    for (i, row) in rows.iter().enumerate() {
        for (j, v) in [row.0, row.1, row.2, row.3, row.4, row.5]
            .iter()
            .enumerate()
        {
            sheet.write_string(i as u32 + 1, j as u16, *v).unwrap();
        }
    }
    workbook.save(path).unwrap();
}
#[test]
fn mapped_import_requires_review_preserves_prices_and_stock_by_default_and_is_atomic() {
    let mut rig = rig();
    let app = &mut rig.app;
    let old = app
        .save_product(Product {
            sku: "OLD".into(),
            name: "Anterior".into(),
            price_cents: 5000,
            cost_cents: Some(3500),
            stock_units: 100000,
            ..Default::default()
        })
        .unwrap();
    let pid = old["id"].as_i64().unwrap();
    let file = rig.directory.path().join("products.xlsx");
    workbook(
        &file,
        &[
            (
                "OLD",
                "Descripción corregida",
                "99.99",
                "2",
                "Pieza",
                "000123",
            ),
            ("NEW", "Cable", "18.50", "3.25", "Metro", "000124"),
        ],
    );
    let info = app.open_import(&file).unwrap();
    assert_eq!(info["rows"], 2);
    assert_eq!(info["mapping"]["sku"], 0);
    let preview = app.validate_import(options()).unwrap();
    assert_eq!(preview["valid"], true);
    assert_eq!(preview["added"], 1);
    assert_eq!(preview["updated"], 1);
    assert_eq!(preview["priceChanges"], 0);
    assert_eq!(
        catalog::product(app.db().unwrap(), pid).unwrap().name,
        "Anterior"
    );
    let id = preview["planId"].as_str().unwrap();
    let result = app.commit_import(id).unwrap();
    assert_eq!(app.commit_import(id).unwrap(), result);
    let p = catalog::product(app.db().unwrap(), pid).unwrap();
    assert_eq!(p.price_cents, 5000);
    assert_eq!(p.cost_cents, Some(3500));
    assert_eq!(p.stock_units, 100000);
    assert_eq!(p.name, "Descripción corregida");
    let new = app.products("NEW", "", "active", 0).unwrap();
    assert_eq!(new["items"][0]["stockUnits"], 32500);
    assert_eq!(new["items"][0]["barcode"], "000124");
    app.open_import(&file).unwrap();
    let mut policy = options();
    policy.mode = "update".into();
    policy.prices = "replace".into();
    policy.stock = "replace".into();
    let preview = app.validate_import(policy).unwrap();
    assert_eq!(preview["valid"], true);
    assert_eq!(preview["priceChanges"], 1);
    assert_eq!(preview["stockChanges"], 1);
    let new_id = new["items"][0]["id"].as_i64().unwrap();
    app.adjust_stock(new_id, 10000, "Movimiento posterior a la vista previa")
        .unwrap();
    assert_eq!(
        app.commit_import(preview["planId"].as_str().unwrap())
            .unwrap_err()
            .code,
        "CATALOG_CHANGED"
    );
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .price_cents,
        5000
    );
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        100000
    );
    db::compatible(app.db().unwrap()).unwrap();
}
#[test]
fn invalid_rows_duplicates_formulas_and_numeric_barcodes_never_write_products() {
    let mut rig = rig();
    let file = rig.directory.path().join("invalid.xlsx");
    workbook(
        &file,
        &[
            ("DUP", "Uno", "100.00", "1", "Pieza", "001"),
            ("DUP", "Dos", "2.50", "1", "Pieza", "002"),
            ("BAD", "Fracción no válida", "4.00", "1.5", "Caja", "003"),
        ],
    );
    rig.app.open_import(&file).unwrap();
    let result = rig.app.validate_import(options()).unwrap();
    assert_eq!(result["valid"], false);
    assert_eq!(result["errorCount"], 2);
    assert_eq!(
        db::scalar(rig.app.db().unwrap(), "SELECT COUNT(*) FROM Product", []).unwrap(),
        0
    );
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    for (i, h) in [
        "Clave",
        "Descripción",
        "Precio con IVA",
        "Existencia",
        "Unidad",
        "Código de barras",
    ]
    .iter()
    .enumerate()
    {
        sheet.write_string(0, i as u16, *h).unwrap();
    }
    sheet
        .write_string(1, 0, "FORMULA")
        .unwrap()
        .write_string(1, 1, "No ejecutar fórmula")
        .unwrap()
        .write_formula(1, 2, Formula::new("1+1"))
        .unwrap()
        .write_string(1, 3, "1")
        .unwrap()
        .write_string(1, 4, "Pieza")
        .unwrap();
    sheet
        .write_string(2, 0, "CODE")
        .unwrap()
        .write_string(2, 1, "Código numérico")
        .unwrap()
        .write_number(2, 2, 20)
        .unwrap()
        .write_number(2, 3, 1)
        .unwrap()
        .write_string(2, 4, "Pieza")
        .unwrap()
        .write_number(2, 5, 123456)
        .unwrap();
    book.save(&file).unwrap();
    rig.app.open_import(&file).unwrap();
    let result = rig.app.validate_import(options()).unwrap();
    assert_eq!(result["valid"], false);
    assert_eq!(result["errorCount"], 2);
    assert!(
        result["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("fórmula")
    );
    assert!(
        result["errors"][1]["message"]
            .as_str()
            .unwrap()
            .contains("texto")
    );
    assert_eq!(
        db::scalar(rig.app.db().unwrap(), "SELECT COUNT(*) FROM Product", []).unwrap(),
        0
    );
    rig.app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(rig.app.open_import(&file).unwrap_err().code, "FORBIDDEN");
}
#[test]
fn oversized_sparse_excel_is_rejected_before_allocating_a_dense_range() {
    let mut rig = rig();
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet
        .write_string(0, 0, "Clave")
        .unwrap()
        .write_string(1_048_575, 16_383, "Demasiado lejos")
        .unwrap();
    let file = rig.directory.path().join("sparse.xlsx");
    book.save(&file).unwrap();
    assert_eq!(rig.app.open_import(&file).unwrap_err().code, "EXCEL_LIMIT");
}
#[test]
fn exported_catalog_preserves_leading_zero_codes_and_can_be_imported_again() {
    let mut rig = rig();
    rig.app
        .save_product(Product {
            sku: "001-TEST".into(),
            barcode: Some("0012345678905".into()),
            name: "Ángulo de acero".into(),
            price_cents: 7999,
            stock_units: 25000,
            unit: 1,
            ..Default::default()
        })
        .unwrap();
    let export = rig
        .app
        .export_products(rig.directory.path(), false)
        .unwrap();
    let path = std::path::PathBuf::from(export["path"].as_str().unwrap());
    let before = donovan_core::storage::hash_file(&path).unwrap();
    let other = rig
        .app
        .export_products(rig.directory.path(), false)
        .unwrap();
    assert_ne!(export["path"], other["path"]);
    assert_eq!(donovan_core::storage::hash_file(&path).unwrap(), before);
    let info = rig.app.open_import(&path).unwrap();
    let mapping = serde_json::from_value(info["mapping"].clone()).unwrap();
    let preview = rig
        .app
        .validate_import(ImportOptions {
            mapping,
            mode: "update".into(),
            prices: "replace".into(),
            stock: "replace".into(),
        })
        .unwrap();
    assert_eq!(preview["valid"], true, "{preview}");
    assert_eq!(preview["priceChanges"], 0);
    assert_eq!(preview["stockChanges"], 0);
    assert_eq!(preview["rows"][0]["priceCents"], 7999);
    assert_eq!(preview["rows"][0]["stockUnits"], 25000);
}
