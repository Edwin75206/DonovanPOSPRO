#![allow(dead_code)]
use donovan_core::{Engine, engine::Clock, models::Setup};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
pub const ADMIN_PASSWORD: &str = "Administracion-prueba-2026";
pub const SELLER_PASSWORD: &str = "Mostrador-prueba-2026";
pub struct TestClock(AtomicI64);
impl Clock for TestClock {
    fn timestamp(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl TestClock {
    pub fn advance(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}
pub struct Rig {
    pub app: Engine,
    pub directory: tempfile::TempDir,
    pub clock: Arc<TestClock>,
}
pub fn rig() -> Rig {
    rig_backend(true)
}
pub fn sqlite_rig() -> Rig {
    rig_backend(false)
}
pub fn reopen(root: &std::path::Path, clock: Arc<TestClock>) -> Engine {
    let app = if let Some(runtime) = std::env::var_os("DONOVAN_TEST_MYSQL_RUNTIME") {
        Engine::with_mysql_clock(root, clock, runtime.into()).unwrap()
    } else {
        Engine::with_clock(root, clock).unwrap()
    };
    assert!(app.startup_error.is_none(), "{:?}", app.startup_error);
    app
}
fn rig_backend(use_mysql: bool) -> Rig {
    let directory = tempfile::TempDir::new().unwrap();
    let clock = Arc::new(TestClock(AtomicI64::new(
        chrono::DateTime::parse_from_rfc3339("2026-10-31T12:00:00-06:00")
            .unwrap()
            .timestamp(),
    )));
    let mut app = if let Some(runtime) =
        std::env::var_os("DONOVAN_TEST_MYSQL_RUNTIME").filter(|_| use_mysql)
    {
        Engine::with_mysql_clock(directory.path(), clock.clone(), runtime.into()).unwrap()
    } else {
        Engine::with_clock(directory.path(), clock.clone()).unwrap()
    };
    assert!(app.startup_error.is_none(), "{:?}", app.startup_error);
    app.setup(Setup {
        store_name: "Ferretería de prueba".into(),
        admin_username: "admin".into(),
        admin_name: "Administración".into(),
        admin_password: ADMIN_PASSWORD.into(),
        seller_username: "negociante.uno".into(),
        seller_name: "Negociante Uno".into(),
        seller_password: SELLER_PASSWORD.into(),
        address: "Domicilio de prueba".into(),
        phone: "".into(),
        tax_id: "".into(),
        ticket_width_mm: 80,
    })
    .unwrap();
    app.login("admin", ADMIN_PASSWORD).unwrap();
    Rig {
        app,
        directory,
        clock,
    }
}
pub fn request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
