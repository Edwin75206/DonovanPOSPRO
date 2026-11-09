//! Explicit native UI acceptance fixture. Never executed by the normal test suite.
#[allow(dead_code)]
#[path = "common/mysql_fixture.rs"]
mod fixture;

#[test]
#[ignore = "requires explicit isolated acceptance root and bundled runtime"]
fn prepare_native_ui_fixture() {
    let root = std::env::var_os("DONOVAN_ACCEPTANCE_ROOT").expect("isolated acceptance root");
    assert!(std::path::Path::new(&root).is_absolute());
    std::env::var_os("DONOVAN_ACCEPTANCE_RUNTIME").expect("bundled acceptance runtime");
    fixture::run();
}
