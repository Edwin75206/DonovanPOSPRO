mod common;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use common::*;
use donovan_core::{
    Engine,
    appearance::Appearance,
    db,
    models::{Checkout, Payment, Product, SaleItem},
    quotes::HeldInput,
};
#[test]
fn locked_session_survives_restart_and_activity_never_extends_24_hours() {
    let Rig {
        mut app,
        directory,
        clock,
    } = rig();
    let expiry = db::scalar(app.db().unwrap(), "SELECT ExpiresAt FROM UserSession", []).unwrap();
    clock.advance(3600);
    app.lock_screen().unwrap();
    assert_eq!(app.actor("sell").unwrap_err().code, "SESSION_LOCKED");
    drop(app);
    let mut app = Engine::with_clock(directory.path(), clock.clone()).unwrap();
    assert_eq!(app.bootstrap().unwrap()["locked"], true);
    assert_eq!(app.actor("sell").unwrap_err().code, "SESSION_LOCKED");
    app.unlock(ADMIN_PASSWORD).unwrap();
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT ExpiresAt FROM UserSession", []).unwrap(),
        expiry
    );
    clock.advance(82799);
    assert_eq!(app.actor("sell").unwrap().username, "admin");
    clock.advance(1);
    assert_eq!(app.actor("sell").unwrap_err().code, "SESSION_EXPIRED");
    app.login("admin", ADMIN_PASSWORD).unwrap();
    let session = std::fs::read(directory.path().join("session.dpapi")).unwrap();
    app.logout().unwrap();
    assert!(!directory.path().join("session.dpapi").exists());
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM UserSession", []).unwrap(),
        0
    );
    // Replaying an old encrypted session file cannot restore a logged-out session.
    drop(app);
    std::fs::write(directory.path().join("session.dpapi"), session).unwrap();
    let app = Engine::with_clock(directory.path(), clock).unwrap();
    assert!(app.bootstrap().unwrap()["actor"].is_null());
}
#[test]
fn backend_permissions_hide_costs_and_restrict_other_cashiers_records() {
    let mut rig = rig();
    let app = &mut rig.app;
    for index in 0..72 {
        app.save_product(Product {
            sku: format!("P{index:03}"),
            name: format!("Tórnilló {index:03}"),
            price_cents: (index + 1) * 100,
            cost_cents: Some(30),
            stock_units: 100000,
            ..Default::default()
        })
        .unwrap();
    }
    let sorted = app
        .product_page("tornillo", "", "active", 0, "priceCents", true)
        .unwrap();
    assert_eq!(sorted["total"], 72);
    assert_eq!(sorted["items"][0]["priceCents"], 7200);
    assert_eq!(sorted["items"][59]["priceCents"], 1300);
    let next = app
        .product_page("tornillo", "", "active", 1, "priceCents", true)
        .unwrap();
    assert_eq!(next["items"][0]["priceCents"], 1200);
    assert_eq!(next["items"].as_array().unwrap().len(), 12);
    assert!(
        app.product_page("", "", "active", 0, "priceCents; DROP TABLE Product", false)
            .is_err()
    );
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Product", []).unwrap(),
        72
    );
    let id = sorted["items"][71usize.min(59)]["id"].as_i64().unwrap();
    let one = SaleItem {
        product_id: id,
        quantity_units: 10000,
        wholesale: false,
    };
    let total = app.price(&[one.clone()], 0).unwrap().total_cents;
    app.cash_open(0).unwrap();
    let admin_sale = app
        .checkout(&Checkout {
            items: vec![one.clone()],
            payments: vec![Payment {
                method: 0,
                amount_cents: total,
                reference: "".into(),
            }],
            tendered_cents: total,
            ..Default::default()
        })
        .unwrap();
    let sid = admin_sale["sale"]["id"].as_i64().unwrap();
    let held = app
        .save_held(HeldInput {
            request_id: request_id(),
            id: None,
            label: "Pedido del administrador".into(),
            items: vec![one.clone()],
            customer_id: None,
            discount_parts: 0,
        })
        .unwrap();
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(app.actor("sell").unwrap().role, 1);
    assert!(
        app.products("", "", "active", 0).unwrap()["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["costCents"].is_null())
    );
    assert!(
        app.price(&[one.clone()], 0).unwrap().lines[0]
            .cost_cents
            .is_none()
    );
    assert_eq!(
        app.product_page("", "", "active", 0, "costCents", false)
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
    assert_eq!(
        app.save_product(Product::default()).unwrap_err().code,
        "FORBIDDEN"
    );
    assert_eq!(
        app.dashboard("2026-10-31", "2026-10-31").unwrap_err().code,
        "FORBIDDEN"
    );
    assert_eq!(app.sale(sid).unwrap_err().code, "FORBIDDEN");
    assert_eq!(
        app.held_sale(held["id"].as_i64().unwrap())
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
    assert_eq!(
        app.price(&[one.clone()], 10000).unwrap_err().code,
        "FORBIDDEN"
    );
    let own = app
        .checkout(&Checkout {
            items: vec![one],
            payments: vec![Payment {
                method: 0,
                amount_cents: total,
                reference: "".into(),
            }],
            tendered_cents: total,
            ..Default::default()
        })
        .unwrap();
    assert!(own["lines"][0]["costCents"].is_null());
    assert_eq!(
        app.sales("2026-10-31", "2026-10-31", 0)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn personal_appearance_is_independent_persistent_and_cannot_change_business_branding() {
    let Rig {
        mut app,
        directory,
        clock,
    } = rig();
    let settings_before = app.bootstrap().unwrap()["settings"].clone();
    let image = STANDARD.encode(include_bytes!(
        "../../../assets/branding/customer-logo-transparent.png"
    ));
    let config = Appearance {
        mode: "light".into(),
        preset: "executive".into(),
        primary: Some("#ffff00".into()),
        font_scale: 150,
        density: "comfortable".into(),
        animations: "none".into(),
        sidebar: "compact".into(),
        background: "image".into(),
        ..Default::default()
    };
    app.save_appearance(config.clone(), "replace", Some(&image))
        .unwrap();
    assert_eq!(app.appearance().unwrap()["backgroundData"], image);
    assert_eq!(app.bootstrap().unwrap()["settings"], settings_before);
    let mut invalid = config.clone();
    invalid.font_scale = 500;
    assert_eq!(
        app.save_appearance(invalid, "keep", None).unwrap_err().code,
        "VALIDATION"
    );
    assert_eq!(
        app.save_appearance(config, "replace", Some(&STANDARD.encode(b"not an image")))
            .unwrap_err()
            .code,
        "INVALID_IMAGE"
    );
    assert_eq!(app.appearance().unwrap()["config"]["fontScale"], 150);
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(app.appearance().unwrap()["config"]["fontScale"], 110);
    assert!(app.appearance().unwrap()["backgroundData"].is_null());
    app.save_appearance(
        Appearance {
            preset: "forest".into(),
            font_scale: 90,
            ..Default::default()
        },
        "remove",
        None,
    )
    .unwrap();
    drop(app);
    let mut app = Engine::with_clock(directory.path(), clock).unwrap();
    assert_eq!(app.actor("sell").unwrap().username, "negociante.uno");
    assert_eq!(app.appearance().unwrap()["config"]["fontScale"], 90);
    app.login("admin", ADMIN_PASSWORD).unwrap();
    assert_eq!(app.appearance().unwrap()["config"]["fontScale"], 150);
    assert_eq!(app.appearance().unwrap()["backgroundData"], image);
    assert_eq!(app.bootstrap().unwrap()["settings"], settings_before);
    app.save_appearance(Appearance::default(), "remove", None)
        .unwrap();
    assert!(app.appearance().unwrap()["backgroundData"].is_null());
    assert_eq!(app.appearance().unwrap()["config"]["fontScale"], 100);
    db::integrity(app.db().unwrap()).unwrap();
}
