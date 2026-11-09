//! Creates isolated records for customer documentation. Never part of the installer.
use donovan_core::{Engine, appearance::Appearance, db, models::Product};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4);
    let root = PathBuf::from(&args[2]).canonicalize().unwrap();
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/customer-documentation")
        .canonicalize()
        .unwrap();
    assert_eq!(root.parent(), Some(parent.as_path()));
    uuid::Uuid::parse_str(root.file_name().unwrap().to_str().unwrap()).unwrap();
    let marker: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(marker["purpose"], "Construrama Chicontla documentation");
    assert!(["seller", "admin"].contains(&args[3].as_str()));
    let data = root.join(format!("data-{}", args[3]));
    if args[1] == "prepare" {
        assert!(!data.exists(), "Never replace an existing database");
        let mut app = Engine::open(&data).unwrap();
        app.seed_demo().unwrap();
        app.db()
            .unwrap()
            .execute(
                "UPDATE StoreSettings SET StoreName='Construrama Chicontla', Address='', Phone=''",
                [],
            )
            .unwrap();
        app.db().unwrap().execute("UPDATE User SET Name=CASE Role WHEN 0 THEN 'Administración' WHEN 1 THEN 'Negociante Uno' ELSE Name END", []).unwrap();
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
        println!("Documentation workspace prepared: {}", args[3]);
    } else {
        assert_eq!(args[1], "verify");
        let app = Engine::open(&data).unwrap();
        let conn = app.db().unwrap();
        db::integrity(conn).unwrap();
        db::compatible(conn).unwrap();
        assert_eq!(
            db::scalar(conn, "SELECT COUNT(*) FROM Sale", []).unwrap(),
            19
        );
        let (folio,total,received,change):(String,i64,i64,i64)=conn.query_row("SELECT Folio,TotalCents,TenderedCents,ChangeCents FROM Sale ORDER BY Id DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!((total, received, change), (56425, 100000, 43575));
        for (sku, quantity) in [("CEM-50", 450000), ("CAB-12", 4445000), ("BRO-2", 310000)] {
            assert_eq!(
                db::scalar(conn, "SELECT StockUnits FROM Product WHERE Sku=?1", [sku]).unwrap(),
                quantity
            );
        }
        let proof = json!({"folio":folio,"totalCents":total,"receivedCents":received,"changeCents":change,"integrity":"ok","stockVerified":true});
        fs::write(
            root.join("operation-verified.json"),
            serde_json::to_vec_pretty(&proof).unwrap(),
        )
        .unwrap();
        println!("{}", proof);
    }
}
