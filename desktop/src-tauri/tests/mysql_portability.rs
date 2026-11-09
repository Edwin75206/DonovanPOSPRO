mod common;
use common::*;
use donovan_core::{
    Engine, db,
    models::{Checkout, Payment, Product, SaleItem},
};
use std::path::PathBuf;

// Requires the pinned official 8.4 runtime. Never connects to a system database.
#[test]
#[ignore = "Run with DONOVAN_TEST_MYSQL_RUNTIME pointing to the verified portable MySQL 8.4 distribution"]
fn mysql_migration_sale_restart_and_restore_on_two_isolated_installations() {
    let runtime =
        PathBuf::from(std::env::var_os("DONOVAN_TEST_MYSQL_RUNTIME").expect("runtime required"));
    let Rig {
        mut app,
        directory,
        clock,
    } = sqlite_rig();
    let p = app
        .save_product(Product {
            sku: "CABLE-12".into(),
            name: "Cable eléctrico calibre 12".into(),
            category: "Electricidad".into(),
            barcode: Some("0012345678905".into()),
            unit: 1,
            price_cents: 1850,
            cost_cents: Some(1000),
            stock_units: 100000,
            ..Default::default()
        })
        .unwrap();
    let id = p["id"].as_i64().unwrap();
    app.cash_open(100000).unwrap();
    let input = Checkout {
        items: vec![SaleItem {
            product_id: id,
            quantity_units: 25000,
            wholesale: false,
        }],
        payments: vec![Payment {
            method: 0,
            amount_cents: 4625,
            reference: "".into(),
        }],
        tendered_cents: 5000,
        ..Default::default()
    };
    let sale = app.checkout(&input).unwrap();
    let folio = sale["sale"]["folio"].clone();
    drop(app);
    let mut app =
        Engine::with_mysql_clock(directory.path(), clock.clone(), runtime.clone()).unwrap();
    assert!(app.startup_error.is_none(), "{:?}", app.startup_error);
    app.migrate_mysql(ADMIN_PASSWORD).unwrap();
    assert!(app.db().unwrap().is_mysql());
    app.login("admin", ADMIN_PASSWORD).unwrap();
    assert_eq!(app.checkout(&input).unwrap()["sale"]["folio"], folio);
    assert_eq!(
        app.products("electrico 12", "Electricidad", "active", 0)
            .unwrap()["total"],
        1
    );
    assert_eq!(
        db::scalar(
            app.db().unwrap(),
            "SELECT StockUnits FROM Product WHERE Id=?1",
            [id]
        )
        .unwrap(),
        75000
    );
    let next = Checkout {
        request_id: request_id(),
        ..input.clone()
    };
    assert_eq!(app.checkout(&next).unwrap()["sale"]["changeCents"], 375);
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        2
    );
    db::compatible(app.db().unwrap()).unwrap();
    let backup = app
        .create_backup(&directory.path().join("backups"))
        .unwrap();
    let second = tempfile::TempDir::new().unwrap();
    let mut target =
        Engine::with_mysql_clock(second.path(), clock.clone(), runtime.clone()).unwrap();
    assert!(target.startup_error.is_none(), "{:?}", target.startup_error);
    let preview = target
        .prepare_recovery(&PathBuf::from(backup["path"].as_str().unwrap()))
        .unwrap();
    target
        .confirm_recovery(preview["planId"].as_str().unwrap(), "admin", ADMIN_PASSWORD)
        .unwrap();
    target.login("admin", ADMIN_PASSWORD).unwrap();
    assert!(target.db().unwrap().is_mysql());
    assert_eq!(target.cash_overview().unwrap()["expectedCents"], 109250);
    assert_eq!(
        db::scalar(
            target.db().unwrap(),
            "SELECT StockUnits FROM Product WHERE Id=?1",
            [id]
        )
        .unwrap(),
        50000
    );
    db::compatible(target.db().unwrap()).unwrap();
    assert!(directory.path().join("donovan.db").exists());
    drop(target);
    drop(app);
    let app = Engine::with_mysql_clock(directory.path(), clock, runtime).unwrap();
    assert!(app.startup_error.is_none(), "{:?}", app.startup_error);
    assert_eq!(app.actor("sell").unwrap().username, "admin");
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        2
    );
    drop(app);
}
