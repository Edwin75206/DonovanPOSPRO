//! Isolated fixtures for the native UI demonstration; never bundled with the app.
use donovan_core::{Engine, appearance::Appearance, db, models::Product, storage};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(
        args.len(),
        4,
        "pos_ui_qa <prepare|verify> <UUID preview directory> <seller|admin>"
    );
    let root = PathBuf::from(&args[2]).canonicalize().unwrap();
    let allowed = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/v2.1/preview")
        .canonicalize()
        .unwrap();
    assert_eq!(root.parent(), Some(allowed.as_path()));
    uuid::Uuid::parse_str(root.file_name().unwrap().to_str().unwrap()).unwrap();
    let marker: Value =
        serde_json::from_slice(&fs::read(root.join("qa-manifest.json")).unwrap()).unwrap();
    assert_eq!(marker["purpose"], "DonovanPOS 2.1 native UI QA");
    assert!(["seller", "admin"].contains(&args[3].as_str()));
    let data = root.join(format!("data-{}", args[3]));
    if args[1] == "prepare" {
        assert!(!data.exists(), "Never overwrite a fixture");
        let mut app = Engine::open(&data).unwrap();
        app.seed_demo().unwrap();
        app.db()
            .unwrap()
            .execute(
                "UPDATE StoreSettings SET StoreName='Construrama Chicontla · DEMO'",
                [],
            )
            .unwrap();
        app.save_product(Product {
            sku: "ESP-10".into(),
            name: "Espátula flexible 10 cm".into(),
            category: "Pintura".into(),
            brand: "Truper".into(),
            price_cents: 3900,
            minimum_units: 50000,
            ..Default::default()
        })
        .unwrap();
        app.demo_login(if args[3] == "seller" { 1 } else { 0 })
            .unwrap();
        app.preferences("skipped", 0, "light", false).unwrap();
        let mut config: Appearance =
            serde_json::from_value(app.appearance().unwrap()["config"].clone()).unwrap();
        config.mode = "light".into();
        app.save_appearance(config, "keep", None).unwrap();
        let before = app.products("", "", "active", 0).unwrap();
        fs::write(root.join(format!("{}-baseline.json",args[3])),serde_json::to_vec_pretty(&json!({"database":storage::display_path(&app.db_path),"saleCount":db::scalar(app.db().unwrap(),"SELECT COUNT(*) FROM Sale",[]).unwrap(),"products":before["items"]})).unwrap()).unwrap();
        println!(
            "Prepared isolated {} fixture at {}",
            args[3],
            data.display()
        );
    } else {
        assert_eq!(args[1], "verify");
        let app = Engine::open(&data).unwrap();
        let baseline: Value = serde_json::from_slice(
            &fs::read(root.join(format!("{}-baseline.json", args[3]))).unwrap(),
        )
        .unwrap();
        let conn = app.db().unwrap();
        db::integrity(conn).unwrap();
        db::compatible(conn).unwrap();
        assert_eq!(
            db::scalar(conn, "SELECT COUNT(*) FROM Sale", []).unwrap(),
            baseline["saleCount"].as_i64().unwrap() + 1
        );
        let (folio,total,received,change):(String,i64,i64,i64)=conn.query_row("SELECT Folio,TotalCents,TenderedCents,ChangeCents FROM Sale ORDER BY Id DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!((total, received, change), (56425, 100000, 43575));
        for (sku, delta) in [("CEM-50", 20000), ("CAB-12", 25000), ("BRO-2", 10000)] {
            let before = baseline["products"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["sku"] == sku)
                .unwrap()["stockUnits"]
                .as_i64()
                .unwrap();
            let now: i64 = conn
                .query_row("SELECT StockUnits FROM Product WHERE Sku=?1", [sku], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(now, before - delta);
        }
        let proof = json!({"folio":folio,"totalCents":total,"receivedCents":received,"changeCents":change,"saleCount":19,"stockVerified":true,"integrity":"ok"});
        fs::write(
            root.join("ui-sale-verified.json"),
            serde_json::to_vec_pretty(&proof).unwrap(),
        )
        .unwrap();
        println!("{}", proof);
    }
}
