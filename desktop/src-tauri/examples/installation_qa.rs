//! Fixtures and assertions for actual NSIS installations. Never bundled in the product.
//! Requires a marked, isolated artifacts/v2/installation/<UUID> directory.
use donovan_core::{
    Engine, catalog, db,
    models::{Product, Setup},
    storage,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
const ADMIN: &str = "Administracion-instalacion-2026";
const SELLER: &str = "Mostrador-instalacion-2026";
fn open(root: &Path, name: &str) -> Engine {
    Engine::open(&root.join(name)).unwrap()
}
fn sales(app: &Engine, count: i64, stock: i64) {
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        count
    );
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT SUM(TotalCents) FROM Sale", []).unwrap(),
        count * 23500
    );
    let product = catalog::product(app.db().unwrap(), 1).unwrap();
    assert_eq!(product.sku, "PRUEBA-PORTABILIDAD");
    assert_eq!(product.stock_units, stock);
    db::compatible(app.db().unwrap()).unwrap();
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(
        args.len(),
        3,
        "usage: installation_qa <prepare-a|restore-b|verify> <QA directory>"
    );
    let root = PathBuf::from(&args[2]).canonicalize().unwrap();
    let allowed = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/v2/installation")
        .canonicalize()
        .unwrap();
    assert_eq!(root.parent(), Some(allowed.as_path()));
    uuid::Uuid::parse_str(root.file_name().unwrap().to_str().unwrap()).unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("qa-manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["purpose"], "DonovanPOS native installer QA");
    for name in ["program-a", "program-b"] {
        let path = root.join(name).join("DonovanPOS.exe");
        if path.exists() {
            assert_eq!(
                storage::hash_file(&path).unwrap(),
                manifest["installedExeSha256"]
            );
            assert!(
                root.join(name)
                    .join("licenses/THIRD_PARTY_NOTICES.txt")
                    .is_file()
            );
            assert!(
                root.join(name)
                    .join("documentation/migrations/004_business_documents.sql")
                    .is_file()
            );
        }
    }
    let result = match args[1].as_str() {
        "prepare-a" => {
            assert!(root.join("program-a/DonovanPOS.exe").is_file());
            assert!(
                !root.join("data-a").exists(),
                "Never reset an existing fixture"
            );
            let mut a = open(&root, "data-a");
            a.setup(Setup {
                store_name: "Chicontla · prueba de instalación".into(),
                admin_username: "admin.instalacion".into(),
                admin_name: "Administración de prueba".into(),
                admin_password: ADMIN.into(),
                seller_username: "negociante.uno".into(),
                seller_name: "Negociante Uno".into(),
                seller_password: SELLER.into(),
                address: "Datos sintéticos de validación".into(),
                phone: "".into(),
                tax_id: "".into(),
                ticket_width_mm: 80,
            })
            .unwrap();
            a.login("admin.instalacion", ADMIN).unwrap();
            a.save_product(Product {
                sku: "PRUEBA-PORTABILIDAD".into(),
                barcode: Some("0012345678905".into()),
                name: "Cemento de prueba".into(),
                price_cents: 23500,
                cost_cents: Some(16000),
                stock_units: 200000,
                ..Default::default()
            })
            .unwrap();
            a.cash_open(100000).unwrap();
            json!({"stage":"prepared-a","businessData":storage::display_path(&a.db_path),"salesExpectedFromUi":1})
        }
        "restore-b" => {
            assert!(root.join("program-b/DonovanPOS.exe").is_file());
            assert!(
                !root.join("data-b").exists(),
                "Never overwrite an existing fixture"
            );
            let mut a = open(&root, "data-a");
            sales(&a, 1, 190000);
            assert_eq!(
                a.bootstrap().unwrap()["actor"]["username"],
                "admin.instalacion"
            );
            let backup = a.create_backup(&root.join("transfer")).unwrap();
            let archive = PathBuf::from(backup["path"].as_str().unwrap());
            storage::read_archive(&archive, None).unwrap();
            let logo = a.bootstrap().unwrap()["settings"]["logoData"].clone();
            let mut b = open(&root, "data-b");
            let plan = b.prepare_recovery(&archive).unwrap();
            assert_eq!(plan["counts"]["sales"], 1);
            let restored = b
                .confirm_recovery(plan["planId"].as_str().unwrap(), "admin.instalacion", ADMIN)
                .unwrap();
            assert!(restored["actor"].is_null());
            assert_eq!(restored["settings"]["logoData"], logo);
            b.login("negociante.uno", SELLER).unwrap();
            sales(&b, 1, 190000);
            json!({"stage":"restored-b","backup":backup,"databaseB":storage::display_path(&b.db_path),"logoMatched":true,"sessionsReset":true,"sellerLoginPassed":true})
        }
        "verify" => {
            let a = open(&root, "data-a");
            let b = open(&root, "data-b");
            sales(&a, 1, 190000);
            sales(&b, 2, 180000);
            assert_ne!(a.db_path, b.db_path);
            assert_eq!(
                b.bootstrap().unwrap()["actor"]["username"],
                "negociante.uno"
            );
            json!({"stage":"verified","salesA":1,"salesB":2,"stockA":19,"stockB":18,"integrity":true,"sellerSessionPersisted":true,"dataDirectoriesIndependent":true})
        }
        _ => panic!("Unknown action"),
    };
    let output = root.join(format!("{}-{}.json", args[1], uuid::Uuid::new_v4()));
    fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
