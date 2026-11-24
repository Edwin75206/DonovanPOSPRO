use donovan_core::{
    Engine, db,
    catalog::InventoryAdjustment,
    models::{Checkout, Payment, Product, ProductAppearance, SaleItem, Setup},
};
use tempfile::TempDir;

fn setup() -> Setup {
    Setup {
        store_name: "Negocio de prueba".into(),
        admin_username: "admin".into(),
        admin_name: "Administración".into(),
        admin_password: "Solo-pruebas-2026".into(),
        seller_username: "vendedor".into(),
        seller_name: "Vendedor".into(),
        seller_password: "Venta-pruebas-2026".into(),
        address: "Calle de prueba".into(),
        phone: "".into(),
        tax_id: "".into(),
        ticket_width_mm: 80,
    }
}

#[test]
fn material_appearance_is_saved_and_returned_in_inventory_and_scanning() {
    let directory = TempDir::new().unwrap();
    let mut engine = match std::env::var_os("DONOVAN_TEST_MYSQL_RUNTIME") {
        Some(runtime) => Engine::open_desktop(directory.path(), runtime.into()),
        None => Engine::open(directory.path()),
    }
    .unwrap();
    let setup = setup();
    let password = setup.admin_password.clone();
    engine.setup(setup).unwrap();
    engine.login("admin", &password).unwrap();
    let appearance = ProductAppearance {
        icon: "cemento".into(),
        color: "azul".into(),
    };
    let result = engine
        .save_product(Product {
            sku: "MATERIAL-01".into(),
            barcode: Some("200000000001".into()),
            name: "Cemento gris".into(),
            stock_units: 10000,
            price_cents: 15000,
            appearance: Some(appearance.clone()),
            ..Default::default()
        })
        .unwrap();
    let id = result["id"].as_i64().unwrap();
    let style = serde_json::to_value(&appearance).unwrap();
    assert_eq!(result["appearance"], style);
    assert_eq!(engine.get_product(id).unwrap()["appearance"], style);
    assert_eq!(engine.product_for_code("200000000001").unwrap()["appearance"], style);
    let page = engine.products("Cemento", "", "active", 0).unwrap();
    assert_eq!(page["items"][0]["appearance"], style);

    let mut existing: Product = serde_json::from_value(engine.get_product(id).unwrap()).unwrap();
    existing.appearance = None;
    assert_eq!(engine.save_product(existing.clone()).unwrap()["appearance"], style);
    existing.appearance = Some(ProductAppearance::default());
    assert!(engine.save_product(existing).unwrap()["appearance"].is_null());
}

#[test]
fn edition_records_stock_without_reason_and_rejects_a_stale_balance() {
    let directory = TempDir::new().unwrap();
    let mut engine = match std::env::var_os("DONOVAN_TEST_MYSQL_RUNTIME") {
        Some(runtime) => Engine::open_desktop(directory.path(), runtime.into()),
        None => Engine::open(directory.path()),
    }
    .unwrap();
    engine.setup(setup()).unwrap();
    engine.login("admin", "Solo-pruebas-2026").unwrap();
    let saved = engine
        .save_product(Product {
            sku: "CASCO".into(),
            name: "Casco protector".into(),
            stock_units: 50_000,
            ..Default::default()
        })
        .unwrap();
    let mut product: Product = serde_json::from_value(saved).unwrap();
    let id = product.id;
    product.stock_units = 70_000;
    product.category = "Seguridad".into();
    let edited = engine
        .save_product_with_stock(
            product.clone(),
            Some(InventoryAdjustment {
                expected_stock_units: 50_000,
                reason: "  ".into(),
            }),
        )
        .unwrap();
    assert_eq!(edited["stockUnits"], 70_000);
    assert_eq!(edited["category"], "Seguridad");
    assert_eq!(engine.adjust_stock(id, -10_000, "").unwrap()["stockUnits"], 60_000);

    product.name = "Cambio con saldo antiguo".into();
    let error = engine
        .save_product_with_stock(
            product,
            Some(InventoryAdjustment {
                expected_stock_units: 70_000,
                reason: String::new(),
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, "STOCK_CHANGED");
    let current = engine.get_product(id).unwrap();
    assert_eq!(current["name"], "Casco protector");
    assert_eq!(current["stockUnits"], 60_000);
    let history = engine.stock_history(id).unwrap();
    assert_eq!(history.as_array().unwrap().len(), 3);
    assert_eq!(history[1]["deltaUnits"], 20_000);
    assert_eq!(history[1]["reason"], "");
}
#[test]
fn first_full_sale_survives_restart_and_retries_without_double_stock() {
    let directory = TempDir::new().unwrap();
    let mut engine = Engine::open(directory.path()).unwrap();
    engine.setup(setup()).unwrap();
    engine.login("admin", "Solo-pruebas-2026").unwrap();
    let product = engine
        .save_product(Product {
            sku: "CABLE-12".into(),
            name: "Cable calibre 12".into(),
            unit: 1,
            price_cents: 1850,
            cost_cents: Some(1000),
            stock_units: 100000,
            ..Default::default()
        })
        .unwrap();
    let id = product["id"].as_i64().unwrap();
    engine.cash_open(100000).unwrap();
    let request = Checkout {
        items: vec![SaleItem {
            product_id: id,
            quantity_units: 25000,
            wholesale: false,
            retail: false,
        }],
        payments: vec![Payment {
            method: 0,
            amount_cents: 4625,
            reference: "".into(),
        }],
        tendered_cents: 5000,
        ..Default::default()
    };
    let sale = engine.checkout(&request).unwrap();
    assert_eq!(sale["sale"]["totalCents"], 4625);
    assert_eq!(sale["sale"]["changeCents"], 375);
    let receipt_id = sale["sale"]["id"].as_i64().unwrap();
    assert_eq!(engine.checkout(&request).unwrap()["sale"]["id"], receipt_id);
    assert_eq!(
        db::scalar(
            engine.db().unwrap(),
            "SELECT StockUnits FROM Product WHERE Id=?1",
            [id]
        )
        .unwrap(),
        75000
    );
    assert_eq!(
        db::scalar(engine.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        1
    );
    drop(engine);
    let engine = Engine::open(directory.path()).unwrap();
    assert_eq!(engine.actor("sell").unwrap().username, "admin");
    assert_eq!(engine.sale(receipt_id).unwrap()["sale"]["totalCents"], 4625);
    assert_eq!(engine.cash_overview().unwrap()["expectedCents"], 104625);
    db::integrity(engine.db().unwrap()).unwrap();
}
