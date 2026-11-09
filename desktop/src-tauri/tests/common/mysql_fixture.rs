//! Creates only an explicitly selected, isolated acceptance-test installation.
//! Never runs automatically in a customer's app or installer.
use donovan_core::{
    Engine, db,
    models::{Product, Setup},
    secure,
};
pub fn run() {
    let root = std::path::PathBuf::from(
        std::env::var_os("DONOVAN_ACCEPTANCE_ROOT").expect("isolated root"),
    );
    let runtime = std::path::PathBuf::from(
        std::env::var_os("DONOVAN_ACCEPTANCE_RUNTIME").expect("bundled runtime"),
    );
    let selected_role = std::env::var("DONOVAN_ACCEPTANCE_ROLE").unwrap_or_else(|_| "admin".into());
    let role = selected_role.as_str();
    std::fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let mut app = Engine::open_desktop(&root, runtime).unwrap();
    assert!(app.startup_error.is_none(), "{:?}", app.startup_error);
    let password_path = app.root.join("acceptance-password.secret");
    if db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM User", []).unwrap() == 0 {
        let password = format!("Aceptacion-{}", uuid::Uuid::new_v4());
        secure::save(&password_path, &password).unwrap();
        app.setup(Setup {
            store_name: "Construrama Chicontla".into(),
            admin_username: "admin".into(),
            admin_name: "Administrador".into(),
            admin_password: password.clone(),
            seller_username: "negociante.uno".into(),
            seller_name: "Negociante Uno".into(),
            seller_password: password.clone(),
            address: "Chicontla, Puebla".into(),
            phone: "".into(),
            tax_id: "".into(),
            ticket_width_mm: 80,
        })
        .unwrap();
        app.login("admin", &password).unwrap();
        for product in [
            Product {
                sku: "CEM-50".into(),
                barcode: Some("7501234500010".into()),
                name: "Cemento gris 50 kg".into(),
                category: "Construcción".into(),
                unit: 4,
                price_cents: 23000,
                stock_units: 240000,
                minimum_units: 50000,
                ..Default::default()
            },
            Product {
                sku: "CAB-12".into(),
                barcode: Some("7501234500027".into()),
                name: "Cable THW calibre 12".into(),
                category: "Electricidad".into(),
                unit: 1,
                price_cents: 1850,
                stock_units: 450000,
                minimum_units: 50000,
                ..Default::default()
            },
            Product {
                sku: "CIN-19".into(),
                barcode: Some("7501234500034".into()),
                name: "Cinta aislante 19 mm".into(),
                category: "Electricidad".into(),
                unit: 0,
                price_cents: 2500,
                stock_units: 30000,
                minimum_units: 30000,
                ..Default::default()
            },
        ] {
            app.save_product(product).unwrap();
        }
        app.cash_open(100000).unwrap();
    }
    let password = secure::load(&password_path)
        .unwrap()
        .expect("acceptance credentials");
    app.login(
        if role == "seller" {
            "negociante.uno"
        } else {
            "admin"
        },
        &password,
    )
    .unwrap();
    db::compatible(app.db().unwrap()).unwrap();
    if std::env::var("DONOVAN_ACCEPTANCE_VERIFY_SALE").as_deref() == Ok("1") {
        use mysql::prelude::Queryable;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let dashboard = app.dashboard(&today, &today).unwrap();
        let overview = app.data_overview().unwrap();
        assert!(dashboard["databaseBytes"].as_u64().unwrap() > 0);
        assert_eq!(dashboard["databaseBytes"], overview["bytes"]);
        let db = app.db().unwrap();
        assert_eq!(db::scalar(db, "SELECT COUNT(*) FROM Sale", []).unwrap(), 1);
        assert_eq!(
            db::scalar(db, "SELECT TotalCents FROM Sale WHERE Folio='V-000001'", []).unwrap(),
            27625
        );
        assert_eq!(
            db::scalar(db, "SELECT ChangeCents FROM Sale", []).unwrap(),
            22375
        );
        assert_eq!(
            db::scalar(db, "SELECT StockUnits FROM Product WHERE Sku='CAB-12'", []).unwrap(),
            425000
        );
        assert_eq!(
            db::scalar(db, "SELECT StockUnits FROM Product WHERE Sku='CEM-50'", []).unwrap(),
            230000
        );
        let settings = app.workbench_connection(&password).unwrap();
        let options = mysql::OptsBuilder::new()
            .ip_or_hostname(Some("127.0.0.1"))
            .tcp_port(settings["port"].as_u64().unwrap() as u16)
            .user(settings["username"].as_str())
            .pass(settings["password"].as_str())
            .db_name(settings["schema"].as_str())
            .prefer_socket(false)
            .server_key_path(Some(root.join("mysql/data/public_key.pem")));
        let mut workbench = mysql::Conn::new(options).unwrap();
        assert_eq!(
            workbench
                .query_first::<i64, _>("SELECT TotalCents FROM Sale WHERE Folio='V-000001'")
                .unwrap(),
            Some(27625)
        );
        let backup = app.create_backup(&root.join("backups")).unwrap();
        std::fs::write(
            root.join("ui-verification.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "sale":"V-000001", "sales":1, "totalCents":27625, "changeCents":22375,
                "cableStockUnits":425000, "cementStockUnits":230000,
                "workbenchAccountReadsSameSale":true, "backup":backup["path"]
            }))
            .unwrap(),
        )
        .unwrap();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &app.data_overview()
                .unwrap_or_else(|_| serde_json::json!({"ready":true,"role":role}))
        )
        .unwrap()
    );
}
