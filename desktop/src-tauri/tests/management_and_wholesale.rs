mod common;

use common::{ADMIN_PASSWORD, SELLER_PASSWORD, request_id, rig};
use donovan_core::{
    contacts::Contact,
    models::{Checkout, Payment, Product, SaleItem},
    quotes::{HeldInput, QuoteInput},
};

fn product(sku: &str) -> Product {
    Product {
        sku: sku.into(),
        name: sku.into(),
        price_cents: 10_000,
        wholesale_cents: 8_000,
        wholesale_minimum_units: Some(100_000),
        stock_units: 250_000,
        ..Product::default()
    }
}

fn item(product_id: i64, quantity_units: i64) -> SaleItem {
    SaleItem { product_id, quantity_units, wholesale: false, retail: false }
}

#[test]
fn wholesale_threshold_and_authorized_overrides_survive_holding() {
    let mut rig = rig();
    let app = &mut rig.app;
    let id = app.save_product(product("P-MAYOREO")).unwrap()["id"].as_i64().unwrap();
    let mut manual = item(id, 50_000);
    manual.wholesale = true;
    assert_eq!(app.price(&[manual.clone()], 0).unwrap().total_cents, 40_000);
    let mut retail = item(id, 100_000);
    retail.retail = true;
    let held = app.save_held(HeldInput {
        request_id: request_id(), id: None, label: "Tarifa manual".into(),
        items: vec![retail.clone()], customer_id: None, discount_parts: 0,
    }).unwrap();
    let restored = app.held_sale(held["id"].as_i64().unwrap()).unwrap();
    assert_eq!(restored["rows"][0]["retail"], true);
    assert_eq!(app.price(&[retail], 0).unwrap().total_cents, 100_000);

    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(app.price(&[item(id, 90_000)], 0).unwrap().total_cents, 90_000);
    assert_eq!(app.price(&[item(id, 100_000)], 0).unwrap().total_cents, 80_000);
    assert_eq!(app.price(&[manual], 0).unwrap_err().code, "FORBIDDEN");
    assert_eq!(app.delete_product(id).unwrap_err().code, "FORBIDDEN");
}

#[test]
fn deletes_unused_records_and_preserves_linked_records() {
    let mut rig = rig();
    let app = &mut rig.app;
    let p = app.save_product(product("P-BORRAR")).unwrap()["id"].as_i64().unwrap();
    let contact = Contact { name: "Contacto temporal".into(), ..Contact::default() };
    let c = app.save_contact(false, contact.clone()).unwrap()["id"].as_i64().unwrap();
    let s = app.save_contact(true, contact).unwrap()["id"].as_i64().unwrap();
    let quote = app.save_quote(QuoteInput {
        request_id: request_id(), id: 0, items: vec![item(p, 10_000)], customer_id: Some(c),
        discount_parts: 0, valid_until: "2026-12-01".into(), notes: String::new(),
    }).unwrap();
    let q = quote["quote"]["id"].as_i64().unwrap();
    assert_eq!(app.delete_product(p).unwrap_err().code, "PRODUCT_IN_USE");
    assert_eq!(app.delete_contact(false, c).unwrap_err().code, "CONTACT_IN_USE");
    app.delete_quote(q).unwrap();
    assert_eq!(app.quote(q).unwrap_err().code, "NOT_FOUND");
    app.delete_contact(false, c).unwrap();
    app.delete_contact(true, s).unwrap();
    app.delete_product(p).unwrap();
    assert_eq!(app.get_product(p).unwrap_err().code, "NOT_FOUND");
    assert_eq!(app.contact(false, c).unwrap_err().code, "NOT_FOUND");
    assert_eq!(app.contact(true, s).unwrap_err().code, "NOT_FOUND");
    let replacement = app.save_product(product("P-BORRAR")).unwrap();
    assert_ne!(replacement["id"], p);
}

#[test]
fn only_latest_sale_can_be_removed_and_its_folio_is_reused() {
    let mut rig = rig();
    let app = &mut rig.app;
    let p = app.save_product(product("P-FOLIO")).unwrap()["id"].as_i64().unwrap();
    app.cash_open(50_000).unwrap();
    let payment = Payment { method: 0, amount_cents: 10_000, reference: String::new() };
    let mut order = Checkout {
        request_id: request_id(),
        items: vec![item(p, 10_000)],
        payments: vec![payment],
        tendered_cents: 10_000,
        ..Checkout::default()
    };
    let first = app.checkout(&order).unwrap();
    order.request_id = request_id();
    let second = app.checkout(&order).unwrap();
    let first_id = first["sale"]["id"].as_i64().unwrap();
    let second_id = second["sale"]["id"].as_i64().unwrap();
    assert_eq!(app.delete_sale(first_id, "Corrección").unwrap_err().code, "SALE_NOT_LATEST");
    assert_eq!(app.sale(second_id).unwrap()["canDelete"], true);
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(app.delete_sale(second_id, "Corrección").unwrap_err().code, "FORBIDDEN");
    app.login("admin", ADMIN_PASSWORD).unwrap();
    let removed = app.delete_sale(second_id, "Corrección de captura").unwrap();
    assert_eq!(removed["folio"], "V-000002");
    assert_eq!(app.get_product(p).unwrap()["stockUnits"], 240_000);
    assert_eq!(app.cash_overview().unwrap()["expectedCents"], 60_000);
    assert_eq!(app.sale(second_id).unwrap_err().code, "NOT_FOUND");
    assert_eq!(app.checkout(&order).unwrap_err().code, "SALE_DELETED");
    order.request_id = request_id();
    let replacement = app.checkout(&order).unwrap();
    assert_eq!(replacement["sale"]["folio"], "V-000002");
    assert_ne!(replacement["sale"]["id"], second_id);
    assert_eq!(app.cash_overview().unwrap()["expectedCents"], 70_000);
}
