mod common;
use common::*;
use donovan_core::{
    Engine,
    appearance::Appearance,
    catalog, db,
    models::{Checkout, Payment, Product, SaleItem},
    storage,
};
use std::{fs, path::PathBuf};
fn sale(product_id: i64, quantity_units: i64, amount: i64) -> Checkout {
    Checkout {
        items: vec![SaleItem {
            product_id,
            quantity_units,
            wholesale: false,
        }],
        payments: vec![Payment {
            method: 0,
            amount_cents: amount,
            reference: "".into(),
        }],
        tendered_cents: amount,
        ..Default::default()
    }
}
#[test]
fn consistent_backup_moves_business_between_two_isolated_data_installations() {
    let mut a = rig();
    let app = &mut a.app;
    let pid = app
        .save_product(Product {
            sku: "CABLE".into(),
            name: "Cable por metro".into(),
            unit: 1,
            stock_units: 100000,
            price_cents: 1850,
            cost_cents: Some(1000),
            ..Default::default()
        })
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    app.cash_open(100000).unwrap();
    app.checkout(&sale(pid, 25000, 4625)).unwrap();
    app.save_appearance(
        Appearance {
            preset: "executive".into(),
            font_scale: 150,
            ..Default::default()
        },
        "keep",
        None,
    )
    .unwrap();
    let settings = app.bootstrap().unwrap()["settings"].clone();
    let hashes = db::all(
        app.db().unwrap(),
        "SELECT Username,PasswordHash,Role FROM User ORDER BY Id",
        [],
    )
    .unwrap();
    assert!(
        fs::metadata(format!("{}-wal", app.db_path.display()))
            .unwrap()
            .len()
            > 0
    );
    let archive = PathBuf::from(
        app.create_backup(&a.directory.path().join("exports"))
            .unwrap()["path"]
            .as_str()
            .unwrap(),
    );
    let manifest = storage::read_archive(&archive, None).unwrap();
    assert!(manifest.database_bytes > 400000);
    let b = tempfile::tempdir().unwrap();
    let mut target = Engine::with_clock(b.path(), a.clock.clone()).unwrap();
    let preview = target.prepare_recovery(&archive).unwrap();
    assert_eq!(preview["counts"]["sales"], 1);
    assert_eq!(preview["counts"]["users"], 2);
    let plan = preview["planId"].as_str().unwrap();
    assert_eq!(
        target
            .confirm_recovery(plan, "admin", "contraseña incorrecta")
            .unwrap_err()
            .code,
        "INVALID_CREDENTIALS"
    );
    assert_eq!(target.bootstrap().unwrap()["needsSetup"], true);
    let restored = target
        .confirm_recovery(plan, "admin", ADMIN_PASSWORD)
        .unwrap();
    assert!(restored["actor"].is_null());
    assert_eq!(restored["needsSetup"], false);
    assert_eq!(
        target
            .confirm_recovery(plan, "admin", ADMIN_PASSWORD)
            .unwrap()["databaseId"],
        restored["databaseId"]
    );
    assert_eq!(restored["settings"]["logoData"], settings["logoData"]);
    assert_eq!(
        db::all(
            target.db().unwrap(),
            "SELECT Username,PasswordHash,Role FROM User ORDER BY Id",
            []
        )
        .unwrap(),
        hashes
    );
    assert_eq!(
        db::scalar(target.db().unwrap(), "SELECT COUNT(*) FROM UserSession", []).unwrap(),
        0
    );
    assert!(target.db_path.starts_with(b.path().canonicalize().unwrap()));
    assert_ne!(target.db_path, a.app.db_path);
    target.login("admin", ADMIN_PASSWORD).unwrap();
    assert_eq!(target.appearance().unwrap()["config"]["fontScale"], 150);
    target.login("negociante.uno", SELLER_PASSWORD).unwrap();
    target.checkout(&sale(pid, 10000, 1850)).unwrap();
    assert_eq!(
        catalog::product(target.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        65000
    );
    assert_eq!(
        catalog::product(a.app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        75000
    );
    assert_eq!(
        db::scalar(target.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        2
    );
    assert_eq!(
        db::scalar(a.app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        1
    );
    let selected = target.db_path.clone();
    drop(target);
    let target = Engine::with_clock(b.path(), a.clock.clone()).unwrap();
    assert_eq!(target.db_path, selected);
    assert_eq!(target.actor("sell").unwrap().username, "negociante.uno");
    db::compatible(target.db().unwrap()).unwrap();
}
#[test]
fn corrupt_incompatible_and_failed_activation_leave_original_business_usable() {
    let mut rig = rig();
    let root = rig.directory.path();
    let app = &mut rig.app;
    let product = app
        .save_product(Product {
            sku: "ORIGINAL".into(),
            name: "Producto original".into(),
            stock_units: 50000,
            price_cents: 100,
            ..Default::default()
        })
        .unwrap();
    let original_id = app.bootstrap().unwrap()["databaseId"].clone();
    let original_path = app.db_path.clone();
    let archive = PathBuf::from(
        app.create_backup(&root.join("exports")).unwrap()["path"]
            .as_str()
            .unwrap(),
    );
    let mut bytes = fs::read(&archive).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0x5a;
    let corrupt = root.join("corrupt.donovanbackup");
    fs::write(&corrupt, bytes).unwrap();
    assert_eq!(
        app.prepare_recovery(&corrupt).unwrap_err().code,
        "INVALID_BACKUP"
    );
    let unrelated = root.join("unrelated.sqlite");
    {
        let db = rusqlite::Connection::open(&unrelated).unwrap();
        db.execute_batch("CREATE TABLE Unrelated(Id INTEGER);")
            .unwrap();
    }
    assert_eq!(
        app.prepare_recovery(&unrelated).unwrap_err().code,
        "INCOMPATIBLE_DATABASE"
    );
    let preview = app.prepare_recovery(&archive).unwrap();
    let pointer = root.join("active.json");
    let before = fs::read(&pointer).unwrap();
    let mut permissions = fs::metadata(&pointer).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&pointer, permissions).unwrap();
    let failed = app.confirm_recovery(preview["planId"].as_str().unwrap(), "admin", ADMIN_PASSWORD);
    let mut permissions = fs::metadata(&pointer).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&pointer, permissions).unwrap();
    assert!(failed.is_err());
    assert_eq!(fs::read(&pointer).unwrap(), before);
    assert_eq!(app.db_path, original_path);
    assert_eq!(app.bootstrap().unwrap()["databaseId"], original_id);
    assert_eq!(app.actor("settings").unwrap().username, "admin");
    assert_eq!(
        catalog::product(app.db().unwrap(), product["id"].as_i64().unwrap())
            .unwrap()
            .stock_units,
        50000
    );
    assert_eq!(
        db::scalar(
            app.db().unwrap(),
            "SELECT COUNT(*) FROM BackupRecord WHERE Kind='Antes de restaurar' AND Successful=1",
            []
        )
        .unwrap(),
        1
    );
    db::compatible(app.db().unwrap()).unwrap();
}
#[test]
fn missing_configured_database_is_never_replaced_with_an_empty_one() {
    let Rig {
        app,
        directory,
        clock,
    } = rig();
    let database = app.db_path.clone();
    drop(app);
    let preserved = directory.path().join("preserved.sqlite");
    fs::rename(&database, &preserved).unwrap();
    let app = Engine::with_clock(directory.path(), clock).unwrap();
    assert_eq!(
        app.bootstrap().unwrap()["storageError"]["code"],
        "STORAGE_MISSING"
    );
    assert!(!database.exists());
    assert!(preserved.exists());
}
#[test]
fn legacy_database_is_copied_migrated_and_keeps_original_accounts_and_file() {
    let mut source = rig();
    source
        .app
        .save_product(Product {
            sku: "LEGACY".into(),
            name: "Producto de la versión anterior".into(),
            stock_units: 30000,
            price_cents: 1234,
            ..Default::default()
        })
        .unwrap();
    let legacy = source.directory.path().join("legacy-v1.sqlite");
    {
        let connection = rusqlite::Connection::open(&legacy).unwrap();
        connection.execute_batch(db::LEGACY_SCHEMA).unwrap();
        connection
            .execute(
                "ATTACH DATABASE ?1 AS modern",
                [storage::display_path(&source.app.db_path)],
            )
            .unwrap();
        let tables:Vec<String>=connection.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'ProductSearch%' AND name NOT LIKE '__EF%'").unwrap().query_map([],|r|r.get(0)).unwrap().collect::<std::result::Result<_,_>>().unwrap();
        for table in tables {
            let columns: Vec<String> = connection
                .prepare(&format!("PRAGMA table_info(\"{table}\")"))
                .unwrap()
                .query_map([], |r| r.get(1))
                .unwrap()
                .collect::<std::result::Result<_, _>>()
                .unwrap();
            let names = columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(",");
            connection
                .execute(
                    &format!(
                        "INSERT INTO \"{table}\"({names}) SELECT {names} FROM modern.\"{table}\""
                    ),
                    [],
                )
                .unwrap();
        }
        connection
            .execute_batch(
                "UPDATE StoreSettings SET DarkMode=1,ReducedMotion=1; DETACH DATABASE modern;",
            )
            .unwrap();
        assert_eq!(db::compatible(&connection).unwrap(), 0);
    }
    let before = storage::hash_file(&legacy).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut target = Engine::with_clock(directory.path(), source.clock.clone()).unwrap();
    let plan = target.prepare_recovery(&legacy).unwrap();
    assert_eq!(plan["legacy"], true);
    target
        .confirm_recovery(plan["planId"].as_str().unwrap(), "admin", ADMIN_PASSWORD)
        .unwrap();
    assert_eq!(storage::hash_file(&legacy).unwrap(), before);
    assert_eq!(
        db::schema(target.db().unwrap()).unwrap(),
        db::SCHEMA_VERSION
    );
    assert_eq!(
        db::scalar(
            target.db().unwrap(),
            "SELECT COUNT(*) FROM UserPreferences WHERE Theme='dark' AND ReducedMotion=1",
            []
        )
        .unwrap(),
        2
    );
    let boot = target.login("admin", ADMIN_PASSWORD).unwrap();
    assert_eq!(boot["actor"]["name"], "Administración");
    assert_eq!(boot["preferences"]["theme"], "dark");
    assert_eq!(
        target.products("LEGACY", "", "active", 0).unwrap()["total"],
        1
    );
    db::compatible(target.db().unwrap()).unwrap();
}
