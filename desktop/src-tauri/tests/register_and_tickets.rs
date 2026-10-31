mod common;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use common::*;
use donovan_core::{
    Engine, db,
    documents::{DocumentRequest, layout::Draw, pdf, printing},
    models::{Checkout, Payment, Product, SaleItem},
};

#[test]
fn schema_four_upgrades_with_safety_copy_and_keeps_commercial_data() {
    let Rig {
        mut app,
        directory,
        clock,
    } = rig();
    let p = app
        .save_product(Product {
            sku: "MIGRAR".into(),
            name: "Mercancía existente".into(),
            price_cents: 23500,
            stock_units: 50000,
            ..Default::default()
        })
        .unwrap();
    app.cash_open(100000).unwrap();
    app.checkout(&Checkout {
        items: vec![SaleItem {
            product_id: p["id"].as_i64().unwrap(),
            quantity_units: 10000,
            wholesale: false,
        }],
        payments: vec![Payment {
            method: 0,
            amount_cents: 23500,
            reference: String::new(),
        }],
        tendered_cents: 25000,
        ..Default::default()
    })
    .unwrap();
    let held = app
        .save_held(donovan_core::quotes::HeldInput {
            request_id: request_id(),
            id: None,
            label: "Carrito que sobrevive al reinicio".into(),
            items: vec![SaleItem {
                product_id: p["id"].as_i64().unwrap(),
                quantity_units: 20000,
                wholesale: false,
            }],
            customer_id: None,
            discount_parts: 0,
        })
        .unwrap();
    app.db().unwrap().execute_batch("DROP TABLE TicketProfile; DELETE FROM NativeMigration WHERE Version=5; PRAGMA user_version=4;").unwrap();
    db::compatible(app.db().unwrap()).unwrap();
    drop(app);
    let mut reopened = Engine::with_clock(directory.path(), clock).unwrap();
    assert!(reopened.startup_error.is_none());
    assert_eq!(db::schema(reopened.db().unwrap()).unwrap(), 5);
    assert_eq!(
        reopened.held_sale(held["id"].as_i64().unwrap()).unwrap()["rows"][0]["quantityUnits"],
        20000
    );
    assert_eq!(
        db::scalar(
            reopened.db().unwrap(),
            "SELECT SUM(TotalCents) FROM Sale",
            []
        )
        .unwrap(),
        23500
    );
    assert_eq!(
        db::scalar(
            reopened.db().unwrap(),
            "SELECT StockUnits FROM Product WHERE Sku='MIGRAR'",
            []
        )
        .unwrap(),
        40000
    );
    let safety: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .flatten()
        .filter(|f| {
            f.file_name()
                .to_string_lossy()
                .starts_with("before-migration-")
        })
        .collect();
    assert_eq!(safety.len(), 1);
    let old = db::connect(&safety[0].path(), true).unwrap();
    assert_eq!(db::schema(&old).unwrap(), 4);
    db::integrity(&old).unwrap();
    let mut config = reopened.ticket_config().unwrap();
    config.width_mm = 58;
    config.margin_mm = 3;
    config.copies = 2;
    config.legend = "Respaldo comercial".into();
    reopened.save_ticket_config(config).unwrap();
    let archive = reopened
        .create_backup(&directory.path().join("exports"))
        .unwrap();
    let destination = tempfile::TempDir::new().unwrap();
    let mut other = Engine::open(destination.path()).unwrap();
    let plan = other
        .prepare_recovery(std::path::Path::new(archive["path"].as_str().unwrap()))
        .unwrap();
    other
        .confirm_recovery(plan["planId"].as_str().unwrap(), "admin", ADMIN_PASSWORD)
        .unwrap();
    other.login("admin", ADMIN_PASSWORD).unwrap();
    let restored = other.ticket_config().unwrap();
    assert_eq!(
        (restored.width_mm, restored.margin_mm, restored.copies),
        (58, 3, 2)
    );
    assert_eq!(restored.legend, "Respaldo comercial");
    db::integrity(other.db().unwrap()).unwrap();
}

#[test]
fn exact_scans_search_categories_and_permissions() {
    let mut rig = rig();
    let app = &mut rig.app;
    let p: Product = serde_json::from_value(
        app.save_product(Product {
            sku: "CEM-50".into(),
            barcode: Some("075000000001".into()),
            name: "Cemento gris".into(),
            category: "Construcción".into(),
            brand: "CEMEX".into(),
            size: "Saco 50 kg".into(),
            price_cents: 23500,
            stock_units: 30000,
            minimum_units: 10000,
            ..Default::default()
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(app.product_for_code("075000000001").unwrap()["id"], p.id);
    assert_eq!(app.product_for_code("cem-50").unwrap()["id"], p.id);
    assert_eq!(
        app.product_for_code("Cemento").unwrap_err().code,
        "CODE_NOT_FOUND"
    );
    for search in ["Cemento", "CEMEX", "Construccion", "50 kg"] {
        assert_eq!(app.products(search, "", "active", 0).unwrap()["total"], 1);
    }
    let by_brand = app
        .products_by_brand("50 kg", "Construcción", "CEMEX", "active", 0)
        .unwrap();
    assert_eq!(by_brand["total"], 1);
    assert_eq!(by_brand["brands"][0]["name"], "CEMEX");
    assert_eq!(
        app.products_by_brand("", "", "Otra marca", "active", 0)
            .unwrap()["total"],
        0
    );
    assert_eq!(
        app.products_by_brand("", "", "' OR 1=1 --", "active", 0)
            .unwrap()["total"],
        0
    );
    assert_eq!(
        app.products_by_brand("", "", "CEMEX", "active", 1).unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        app.products("", "Construcción", "active", 0).unwrap()["total"],
        1
    );
    let mut renamed = p.clone();
    renamed.category = "Materiales pesados".into();
    app.save_product(renamed).unwrap();
    assert_eq!(
        app.products("", "", "active", 0).unwrap()["categories"][0]["name"],
        "Materiales pesados"
    );
    app.save_product(Product {
        sku: "VACIO".into(),
        barcode: Some("000000000002".into()),
        name: "Agotado".into(),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        app.product_for_code("000000000002").unwrap_err().code,
        "INSUFFICIENT_STOCK"
    );
    app.save_product(Product {
        sku: "ARCHIVO".into(),
        barcode: Some("000000000003".into()),
        name: "Archivado".into(),
        stock_units: 10000,
        active: false,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        app.product_for_code("000000000003").unwrap_err().code,
        "PRODUCT_UNAVAILABLE"
    );
    app.save_product(Product {
        sku: "075000000001".into(),
        name: "Clave ambigua".into(),
        stock_units: 10000,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        app.product_for_code("075000000001").unwrap_err().code,
        "AMBIGUOUS_CODE"
    );
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert!(app.product_for_code("CEM-50").unwrap()["costCents"].is_null());
    assert_eq!(
        app.adjust_stock(p.id, 10000, "Prueba").unwrap_err().code,
        "FORBIDDEN"
    );
}

#[test]
fn mixed_checkout_rechecks_stock_and_failed_print_never_recharges() {
    let mut rig = rig();
    let app = &mut rig.app;
    app.cash_open(100000).unwrap();
    let mut items = Vec::new();
    for (sku, name, unit, price, stock, qty) in [
        ("CEM", "Cemento", 0, 23500, 30000, 20000),
        ("CAB", "Cable", 1, 1850, 100000, 25000),
        ("BRO", "Brocha", 0, 4800, 30000, 10000),
    ] {
        let p = app
            .save_product(Product {
                sku: sku.into(),
                name: name.into(),
                unit,
                price_cents: price,
                stock_units: stock,
                ..Default::default()
            })
            .unwrap();
        items.push(SaleItem {
            product_id: p["id"].as_i64().unwrap(),
            quantity_units: qty,
            wholesale: false,
        });
    }
    let request = Checkout {
        items: items.clone(),
        payments: vec![
            Payment {
                method: 0,
                amount_cents: 40000,
                reference: String::new(),
            },
            Payment {
                method: 1,
                amount_cents: 16425,
                reference: "terminal manual".into(),
            },
        ],
        tendered_cents: 50000,
        ..Default::default()
    };
    let sale = app.checkout(&request).unwrap();
    assert_eq!(sale["sale"]["totalCents"], 56425);
    assert_eq!(sale["sale"]["changeCents"], 10000);
    assert_eq!(
        app.checkout(&request).unwrap()["sale"]["id"],
        sale["sale"]["id"]
    );
    let document = app
        .business_document(&DocumentRequest {
            kind: "sale".into(),
            id: sale["sale"]["id"].as_i64().unwrap(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        printing::print(&document, "Donovan QA - impresora inexistente", None)
            .unwrap_err()
            .code,
        "PRINTER_MISSING"
    );
    assert!(pdf::encode(&document).unwrap().starts_with(b"%PDF"));
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        1
    );
    assert_eq!(
        app.get_product(items[0].product_id).unwrap()["stockUnits"],
        10000
    );
    assert_eq!(
        app.get_product(items[1].product_id).unwrap()["stockUnits"],
        75000
    );
    let second = Checkout {
        request_id: request_id(),
        ..request.clone()
    };
    assert_eq!(
        app.checkout(&second).unwrap_err().code,
        "INSUFFICIENT_STOCK"
    );
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        1
    );
    let (status, offline) = printing::status_message(0x80, 0);
    assert!(offline);
    assert!(status.contains("conexión"));
    assert!(!printing::status_message(0, 0).0.contains("Conectada"));
    db::integrity(app.db().unwrap()).unwrap();
}

#[test]
fn ticket_settings_preview_monochrome_and_no_printer_are_persistent_and_safe() {
    let mut rig = rig();
    let app = &mut rig.app;
    let original = app.bootstrap().unwrap()["settings"]["logoData"]
        .as_str()
        .unwrap()
        .to_string();
    let mut config = app.ticket_config().unwrap();
    config.width_mm = 58;
    config.margin_mm = 3;
    config.copies = 2;
    config.footer = "Gracias por construir con nosotros.".into();
    config.legend = "Cambios con este comprobante.".into();
    config.monochrome = true;
    app.save_ticket_config(config.clone()).unwrap();
    assert_eq!(app.ticket_config().unwrap().copies, 2);
    let document = app
        .business_document(&DocumentRequest {
            kind: "ticketTest".into(),
            id: 0,
            ..Default::default()
        })
        .unwrap();
    assert!(document.pages[0].width < 165.);
    assert!((document.pages[0].margin - 3. * 72. / 25.4).abs() < 0.01);
    let data = STANDARD
        .decode(document.logo_data.as_ref().unwrap())
        .unwrap();
    let image = image::load_from_memory(&data).unwrap().to_rgba8();
    assert!(
        image
            .pixels()
            .all(|p| p[0] == p[1] && p[1] == p[2] && (p[0] == 0 || p[0] == 255))
    );
    assert_eq!(app.bootstrap().unwrap()["settings"]["logoData"], original);
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        0
    );
    for page in &document.pages {
        for op in &page.operations {
            if let Draw::Text { x, width, .. } = op {
                assert!(*x >= page.margin - 0.2);
                assert!(x + width <= page.width - page.margin + 0.2);
            }
        }
    }
    if let Ok(folder) = std::env::var("DONOVAN_REGISTER_QA_OUTPUT") {
        std::fs::write(
            std::path::Path::new(&folder).join("prueba-ticket-58.pdf"),
            pdf::encode(&document).unwrap(),
        )
        .unwrap();
    }
    config.width_mm = 80;
    config.margin_mm = 4;
    app.save_ticket_config(config.clone()).unwrap();
    let document80 = app
        .business_document(&DocumentRequest {
            kind: "ticketTest".into(),
            id: 0,
            ..Default::default()
        })
        .unwrap();
    if let Ok(folder) = std::env::var("DONOVAN_REGISTER_QA_OUTPUT") {
        let root = std::path::Path::new(&folder);
        std::fs::write(
            root.join("prueba-ticket-80.pdf"),
            pdf::encode(&document80).unwrap(),
        )
        .unwrap();
        printing::print_copies(
            &document80,
            "Microsoft Print to PDF",
            Some(&root.join("prueba-gdi-dos-copias.pdf")),
            2,
        )
        .unwrap();
    }
    config.auto_print = true;
    config.printer_name = "No existe".into();
    assert_eq!(
        app.save_ticket_config(config.clone()).unwrap_err().code,
        "PRINTER_MISSING"
    );
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(
        app.save_ticket_config(config).unwrap_err().code,
        "FORBIDDEN"
    );
    assert_eq!(
        app.business_document(&DocumentRequest {
            kind: "ticketTest".into(),
            id: 0,
            ..Default::default()
        })
        .unwrap_err()
        .code,
        "FORBIDDEN"
    );
    db::integrity(app.db().unwrap()).unwrap();
}
