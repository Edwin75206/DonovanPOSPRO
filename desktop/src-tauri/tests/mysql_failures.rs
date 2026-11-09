mod common;
use common::*;
use donovan_core::{Engine, db, mysql_runtime};
use mysql::prelude::Queryable;

#[test]
#[ignore = "Requires the verified local MySQL runtime in DONOVAN_TEST_MYSQL_RUNTIME"]
fn mysql_permissions_schema_changes_and_lost_commit_fail_closed() {
    let runtime = std::path::PathBuf::from(
        std::env::var_os("DONOVAN_TEST_MYSQL_RUNTIME").expect("runtime required"),
    );
    let Rig {
        mut app,
        directory,
        clock,
    } = rig();
    let details = app.workbench_connection(ADMIN_PASSWORD).unwrap();
    let opts = mysql::OptsBuilder::new()
        .ip_or_hostname(Some("127.0.0.1"))
        .tcp_port(details["port"].as_u64().unwrap() as u16)
        .user(Some("donovan_admin"))
        .pass(Some(details["password"].as_str().unwrap()))
        .db_name(Some(details["schema"].as_str().unwrap()))
        .prefer_socket(false)
        .server_key_path(Some(app.root.join("mysql/data/public_key.pem")));
    let mut workbench = mysql::Conn::new(opts).unwrap();
    let count: Option<i64> = workbench.query_first("SELECT COUNT(*) FROM User").unwrap();
    assert_eq!(count, Some(2));
    assert!(
        app.db()
            .unwrap()
            .execute("CREATE TABLE Unauthorized(Id BIGINT)", [])
            .is_err()
    );
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(
        app.workbench_connection(SELLER_PASSWORD).unwrap_err().code,
        "FORBIDDEN"
    );
    app.login("admin", ADMIN_PASSWORD).unwrap();
    workbench
        .query_drop("ALTER TABLE Product ADD COLUMN Unauthorized BIGINT NULL")
        .unwrap();
    assert_eq!(
        db::compatible(app.db().unwrap()).unwrap_err().code,
        "INCOMPATIBLE_DATABASE"
    );
    workbench
        .query_drop("ALTER TABLE Product DROP COLUMN Unauthorized")
        .unwrap();
    db::compatible(app.db().unwrap()).unwrap();
    let id = db::scalar(app.db().unwrap(), "SELECT CONNECTION_ID()", []).unwrap();
    let tx = app.db().unwrap().unchecked_transaction().unwrap();
    tx.execute(
        "INSERT INTO AppMeta(Key,Value) VALUES('uncommitted-test','must-not-survive')",
        [],
    )
    .unwrap();
    let mut maintenance = mysql_runtime::root_connection(&app.root, None).unwrap();
    maintenance
        .query_drop(format!("KILL CONNECTION {id}"))
        .unwrap();
    assert!(tx.commit().is_err());
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM User", [])
            .unwrap_err()
            .code,
        "STORAGE_UNAVAILABLE"
    );
    drop(workbench);
    drop(maintenance);
    drop(app);
    let app = Engine::with_mysql_clock(directory.path(), clock, runtime).unwrap();
    assert!(app.startup_error.is_none(), "{:?}", app.startup_error);
    assert_eq!(
        db::scalar(
            app.db().unwrap(),
            "SELECT COUNT(*) FROM AppMeta WHERE Key='uncommitted-test'",
            []
        )
        .unwrap(),
        0
    );
    assert_eq!(app.actor("sell").unwrap().username, "admin");
    db::compatible(app.db().unwrap()).unwrap();
    drop(app);
}
