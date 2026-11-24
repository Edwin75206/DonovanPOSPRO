mod common;

use common::{request_id, rig};
use donovan_core::{
    contacts::Contact,
    db,
    models::{BoxContent, Checkout, Payment, Product, SaleItem},
    purchasing::{PurchaseInput, PurchaseItem},
    quotes::QuoteInput,
    returns::ReturnInput,
};

#[test]
fn receiving_and_selling_a_box_share_inventory_with_its_contents() {
    let mut rig = rig();
    let app = &mut rig.app;
    let piece = app.save_product(Product {
        sku: "CEM".into(), barcode: Some("200000000001".into()), name: "Cemento".into(),
        price_cents: 20000, ..Product::default()
    }).unwrap()["id"].as_i64().unwrap();
    let product = Product {
        sku: "CEM-10".into(), barcode: Some("200000001010".into()), name: "Caja de 10 cementos".into(),
        unit: 4, price_cents: 180000, box_content: Some(BoxContent { product_id: piece, quantity: 10 }),
        ..Product::default()
    };
    let package = app.save_product(product.clone()).unwrap()["id"].as_i64().unwrap();
    assert_eq!(app.get_product(package).unwrap()["stockUnits"], 0);
    assert_eq!(app.delete_product(piece).unwrap_err().code, "PRODUCT_IN_USE");
    let supplier = app.save_contact(true, Contact {
        name: "Distribuidor".into(), ..Contact::default()
    }).unwrap()["id"].as_i64().unwrap();
    let date = app.now_string()[..10].to_owned();
    let purchase = app.receive_purchase(PurchaseInput {
        request_id: request_id(), supplier_id: supplier, reference: "CAJA-1".into(),
        date, paid_from_cash: false,
        items: vec![PurchaseItem { product_id: package, quantity_units: 10000, unit_cost_cents: 150000, by_pack: false }],
    }).unwrap();
    assert_eq!(purchase["totalCents"], 150000);
    assert_eq!(app.get_product(piece).unwrap()["stockUnits"], 100000);
    assert_eq!(app.get_product(piece).unwrap()["costCents"], 15000);
    let scanned = app.product_for_code("200000001010").unwrap();
    assert_eq!(scanned["id"], package);
    assert_eq!(scanned["stockUnits"], 10000);
    assert_eq!(scanned["priceCents"], 180000);
    let listed = app.product_page("CEM-10", "", "active", 0, "stockUnits", false).unwrap();
    assert_eq!(listed["items"][0]["stockUnits"], 10000);
    let empty = app.product_page("", "", "empty", 0, "stockUnits", false).unwrap();
    assert_eq!(empty["total"], 0);

    app.cash_open(0).unwrap();
    let item = |product_id| SaleItem { product_id, quantity_units: 10000, wholesale: false, retail: false };
    let checkout = |items, amount_cents| Checkout {
        request_id: request_id(), items, tendered_cents: amount_cents,
        payments: vec![Payment { method: 0, amount_cents, reference: String::new() }],
        ..Checkout::default()
    };
    assert_eq!(app.checkout(&checkout(vec![item(package), item(piece)], 200000)).unwrap_err().code, "INSUFFICIENT_STOCK");
    assert_eq!(app.get_product(piece).unwrap()["stockUnits"], 100000);
    assert_eq!(app.cash_overview().unwrap()["expectedCents"], 0);
    let sale = app.checkout(&checkout(vec![item(package)], 180000)).unwrap();
    assert_eq!(app.get_product(piece).unwrap()["stockUnits"], 0);
    assert_eq!(sale["lines"][0]["unit"], 4);
    app.return_sale(ReturnInput {
        request_id: request_id(), sale_id: sale["sale"]["id"].as_i64().unwrap(),
        reason: "Reintegro de caja".into(), method: 0, items: vec![], cancel: true,
    }).unwrap();
    assert_eq!(app.get_product(piece).unwrap()["stockUnits"], 100000);
    assert_eq!(app.get_product(package).unwrap()["stockUnits"], 10000);
    let removable = app.save_product(Product { sku: "BOLSA".into(), name: "Bolsa sin operaciones".into(), ..Product::default() })
        .unwrap()["id"].as_i64().unwrap();
    assert_eq!(app.delete_product(removable).unwrap()["deleted"], true);
    db::integrity(app.db().unwrap()).unwrap();
}

#[test]
fn first_quote_after_an_old_empty_sequence_starts_at_one() {
    let mut rig = rig();
    let app = &mut rig.app;
    let piece = app.save_product(Product { sku: "CEM".into(), name: "Cemento".into(), price_cents: 20000, ..Product::default() })
        .unwrap()["id"].as_i64().unwrap();
    app.db().unwrap().execute("INSERT INTO Sequence(Key,Value) VALUES ('quotes',1)", []).unwrap();
    let quote = app.save_quote(QuoteInput {
        request_id: request_id(), id: 0,
        items: vec![SaleItem { product_id: piece, quantity_units: 10000, wholesale: false, retail: false }],
        customer_id: None, discount_parts: 0, valid_until: "2026-12-31".into(), notes: String::new(),
    }).unwrap();
    assert_eq!(quote["quote"]["folio"], "CT-000001");
}
