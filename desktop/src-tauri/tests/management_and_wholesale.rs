mod common;

use common::{ADMIN_PASSWORD, SELLER_PASSWORD, request_id, rig};
use donovan_core::{
    contacts::Contact,
    documents::{DocumentRequest, layout::Draw},
    history::SalesQuery,
    models::{Checkout, Payment, Product, SaleItem},
    quotes::{HeldInput, QuoteInput},
    returns::ReturnInput,
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

#[test]
fn deleting_the_last_quote_reuses_its_folio_but_deleting_an_older_one_keeps_the_sequence() {
    let mut rig = rig();
    let app = &mut rig.app;
    let id = app.save_product(product("P-COTIZAR")).unwrap()["id"].as_i64().unwrap();
    let input = |request_id| QuoteInput {
        request_id, id: 0, items: vec![item(id, 10_000)], customer_id: None,
        discount_parts: 0, valid_until: "2026-12-01".into(), notes: String::new(),
    };
    let first = app.save_quote(input(request_id())).unwrap();
    let second = app.save_quote(input(request_id())).unwrap();
    let original_request = input(request_id());
    let third = app.save_quote(original_request.clone()).unwrap();
    let quote_id = |quote: &serde_json::Value| quote["quote"]["id"].as_i64().unwrap();

    let removed_middle = app.delete_quote(quote_id(&second)).unwrap();
    assert!(removed_middle["reusableFolio"].is_null());
    let fourth = app.save_quote(input(request_id())).unwrap();
    assert_eq!(fourth["quote"]["folio"], "CT-000004");
    assert_eq!(
        app.delete_quote(quote_id(&fourth)).unwrap()["reusableFolio"],
        "CT-000004"
    );
    assert_eq!(
        app.delete_quote(quote_id(&third)).unwrap()["reusableFolio"],
        "CT-000003"
    );
    let replacement = app.save_quote(input(request_id())).unwrap();
    assert_eq!(replacement["quote"]["folio"], "CT-000003");
    assert_ne!(quote_id(&replacement), quote_id(&third));
    assert_eq!(app.save_quote(original_request).unwrap_err().code, "NOT_FOUND");
    app.delete_quote(quote_id(&replacement)).unwrap();
    assert_eq!(
        app.delete_quote(quote_id(&first)).unwrap()["reusableFolio"],
        "CT-000001"
    );
    assert_eq!(
        app.save_quote(input(request_id())).unwrap()["quote"]["folio"], "CT-000001"
    );
}

#[test]
fn cancelling_an_earlier_sale_keeps_the_folio_and_reverses_cash_and_stock_once() {
    let mut rig = rig();
    let app = &mut rig.app;
    let sku = app.save_product(product("P-CANCELAR")).unwrap()["id"].as_i64().unwrap();
    app.cash_open(50_000).unwrap();
    let sale = |method| Checkout {
        request_id: request_id(),
        items: vec![item(sku, 10_000)],
        payments: vec![Payment { method, amount_cents: 10_000, reference: String::new() }],
        tendered_cents: if method == 0 { 10_000 } else { 0 },
        ..Checkout::default()
    };
    app.checkout(&sale(0)).unwrap();
    let second = app.checkout(&sale(0)).unwrap();
    app.checkout(&sale(1)).unwrap();
    let second_id = second["sale"]["id"].as_i64().unwrap();
    assert_eq!(app.get_product(sku).unwrap()["stockUnits"], 220_000);
    assert_eq!(app.cash_overview().unwrap()["expectedCents"], 70_000);

    let cancel = ReturnInput {
        request_id: request_id(),
        sale_id: second_id,
        items: vec![],
        reason: "Corrección de la venta".into(),
        method: 0,
        cancel: true,
    };
    let result = app.return_sale_checked(cancel.clone(), Some((10_000, 10_000))).unwrap();
    assert_eq!(app.return_sale(cancel).unwrap(), result);
    assert_eq!(result["cancelled"], true);
    let drawer = app.cash_overview().unwrap();
    assert_eq!(drawer["expectedCents"], 60_000);
    assert_eq!(drawer["movements"][0]["kind"], "Cancelación");
    assert_eq!(drawer["movements"][0]["reference"], result["return"]["folio"]);
    assert_eq!(drawer["movements"][0]["amountCents"], -10_000);
    assert_eq!(app.get_product(sku).unwrap()["stockUnits"], 230_000);

    let retained = app.sale(second_id).unwrap();
    assert_eq!(retained["sale"]["folio"], "V-000002");
    assert_eq!(retained["sale"]["cancelled"], true);
    assert_eq!(retained["canDelete"], false);
    let date = app.now_string()[..10].to_owned();
    let cancelled = app.sales_page(SalesQuery {
        from: date.clone(), to: date, search: String::new(), method: None,
        status: "cancelled".into(), page: 0, sort: "folio".into(), descending: true,
    }).unwrap();
    assert_eq!(cancelled["total"], 1);
    assert_eq!(cancelled["items"][0]["folio"], "V-000002");
    let ticket = app.business_document(&DocumentRequest {
        kind: "sale".into(), id: second_id, width_mm: None, ticket: None, copies: None, report: None,
    }).unwrap();
    assert!(ticket.pages.iter().flat_map(|page| &page.operations)
        .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "CANCELADA")));
    assert_eq!(app.checkout(&sale(0)).unwrap()["sale"]["folio"], "V-000004");
}
