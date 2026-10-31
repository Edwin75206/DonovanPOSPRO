mod common;
use common::*;
use donovan_core::{
    catalog,
    contacts::{Contact, CreditPayment},
    db,
    models::{Checkout, Payment, Product, SaleItem},
    purchasing::{PurchaseInput, PurchaseItem},
    quotes::{HeldInput, QuoteInput},
    returns::{ReturnInput, ReturnItem},
};
fn item(id: i64, quantity: i64) -> SaleItem {
    SaleItem {
        product_id: id,
        quantity_units: quantity,
        wholesale: false,
    }
}
fn payment(method: i64, cents: i64) -> Payment {
    Payment {
        method,
        amount_cents: cents,
        reference: String::new(),
    }
}
#[test]
fn purchase_quote_mixed_credit_refunds_and_suspended_sale_preserve_business_rules() {
    let mut rig = rig();
    let app = &mut rig.app;
    app.cash_open(100000).unwrap();
    let p = app
        .save_product(Product {
            sku: "TOR-1".into(),
            name: "Tornillo de prueba".into(),
            price_cents: 100,
            cost_cents: Some(100),
            stock_units: 100000,
            pack_units: 50000,
            ..Default::default()
        })
        .unwrap();
    let pid = p["id"].as_i64().unwrap();
    let supplier = app
        .save_contact(
            true,
            Contact {
                name: "Proveedor de prueba".into(),
                ..Default::default()
            },
        )
        .unwrap();
    let purchase = PurchaseInput {
        request_id: request_id(),
        supplier_id: supplier["id"].as_i64().unwrap(),
        reference: "FACTURA-1".into(),
        date: "2026-10-31".into(),
        paid_from_cash: false,
        items: vec![PurchaseItem {
            product_id: pid,
            quantity_units: 20000,
            unit_cost_cents: 250,
            by_pack: true,
        }],
    };
    let received = app.receive_purchase(purchase.clone()).unwrap();
    assert_eq!(received["totalCents"], 500);
    assert_eq!(
        app.receive_purchase(purchase.clone()).unwrap()["id"],
        received["id"]
    );
    let mut altered = purchase.clone();
    altered.items[0].unit_cost_cents = 251;
    assert_eq!(
        app.receive_purchase(altered).unwrap_err().code,
        "REQUEST_CHANGED"
    );
    let mut duplicate = purchase.clone();
    duplicate.request_id = request_id();
    assert_eq!(
        app.receive_purchase(duplicate).unwrap_err().code,
        "DUPLICATE_PURCHASE"
    );
    let product = catalog::product(app.db().unwrap(), pid).unwrap();
    assert_eq!(product.stock_units, 200000);
    assert_eq!(product.cost_cents, Some(75));
    let mut impossible = purchase;
    impossible.request_id = request_id();
    impossible.reference = "FACTURA-2".into();
    impossible.paid_from_cash = true;
    impossible.items[0].unit_cost_cents = 100000;
    assert_eq!(
        app.receive_purchase(impossible).unwrap_err().code,
        "INSUFFICIENT_CASH"
    );
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        200000
    );
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Purchase", []).unwrap(),
        1
    );
    let customer = app
        .save_contact(
            false,
            Contact {
                name: "Cliente de prueba".into(),
                credit_limit_cents: 10000,
                ..Default::default()
            },
        )
        .unwrap();
    let cid = customer["id"].as_i64().unwrap();
    let quoted = app
        .save_quote(QuoteInput {
            request_id: request_id(),
            id: 0,
            items: vec![item(pid, 30000)],
            customer_id: Some(cid),
            discount_parts: 333333,
            valid_until: "2026-11-30".into(),
            notes: "Precio acordado".into(),
        })
        .unwrap();
    let qid = quoted["quote"]["id"].as_i64().unwrap();
    assert_eq!(quoted["quote"]["totalCents"], 200);
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        200000
    );
    let mut changed = product;
    changed.price_cents = 250;
    app.save_product(changed).unwrap();
    let request = Checkout {
        quote_id: Some(qid),
        payments: vec![payment(0, 80), payment(3, 120)],
        tendered_cents: 100,
        customer_id: Some(cid),
        due_date: Some("2026-11-15".into()),
        ..Default::default()
    };
    let sold = app.checkout(&request).unwrap();
    assert_eq!(sold["sale"]["totalCents"], 200);
    assert_eq!(sold["sale"]["changeCents"], 20);
    assert_eq!(sold["lines"][0]["priceCents"], 100);
    let sid = sold["sale"]["id"].as_i64().unwrap();
    let lid = sold["lines"][0]["id"].as_i64().unwrap();
    let mut second = request.clone();
    second.request_id = request_id();
    assert_eq!(app.checkout(&second).unwrap_err().code, "ALREADY_COMPLETED");
    let mut altered_sale = request.clone();
    altered_sale.tendered_cents = 200;
    assert_eq!(
        app.checkout(&altered_sale).unwrap_err().code,
        "REQUEST_CHANGED"
    );
    let abono = CreditPayment {
        request_id: request_id(),
        customer_id: cid,
        sale_id: sid,
        amount_cents: 20,
        method: 0,
        reference: "".into(),
    };
    let paid = app.credit_payment(abono.clone()).unwrap();
    assert_eq!(paid["balanceCents"], 100);
    assert_eq!(app.credit_payment(abono).unwrap()["id"], paid["id"]);
    let mut refunds = vec![];
    for (index, amount) in [67, 66, 67].iter().enumerate() {
        let refund = ReturnInput {
            request_id: request_id(),
            sale_id: sid,
            items: vec![ReturnItem {
                sale_line_id: lid,
                quantity_units: 10000,
            }],
            reason: "Producto devuelto".into(),
            method: 0,
            cancel: index == 2,
        };
        let result = app.return_sale(refund.clone()).unwrap();
        assert_eq!(result["return"]["totalCents"], *amount);
        assert_eq!(
            app.return_sale(refund).unwrap()["return"]["id"],
            result["return"]["id"]
        );
        refunds.push(result);
    }
    assert_eq!(refunds[0]["return"]["creditReductionCents"], 67);
    assert_eq!(refunds[1]["return"]["creditReductionCents"], 33);
    assert_eq!(refunds[1]["paidRefundCents"], 33);
    assert_eq!(refunds[2]["paidRefundCents"], 67);
    assert_eq!(app.cash_overview().unwrap()["expectedCents"], 100000);
    assert_eq!(app.customer_ledger(cid).unwrap()["balanceCents"], 0);
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        200000
    );
    let held = app
        .save_held(HeldInput {
            request_id: request_id(),
            id: None,
            label: "Pasará más tarde".into(),
            items: vec![item(pid, 20000)],
            customer_id: None,
            discount_parts: 0,
        })
        .unwrap();
    let hid = held["id"].as_i64().unwrap();
    assert_eq!(
        app.held_sale(hid).unwrap()["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        200000
    );
    let resumed = Checkout {
        held_sale_id: Some(hid),
        items: vec![item(pid, 20000)],
        payments: vec![payment(0, 500)],
        tendered_cents: 500,
        ..Default::default()
    };
    app.checkout(&resumed).unwrap();
    assert!(app.held_sales().unwrap().as_array().unwrap().is_empty());
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        180000
    );
    let dashboard = app.dashboard("2026-10-31", "2026-10-31").unwrap();
    assert_eq!(dashboard["totalCents"], 500);
    assert_eq!(dashboard["returnsCents"], 200);
    assert_eq!(dashboard["receivableCents"], 0);
    assert_eq!(dashboard["cashCents"], 100500);
    assert_eq!(dashboard["operations"], 2);
    db::integrity(app.db().unwrap()).unwrap();
}
#[test]
fn failed_sale_rolls_back_every_line_and_cash_refund_rolls_back_inventory() {
    let mut rig = rig();
    let app = &mut rig.app;
    app.cash_open(0).unwrap();
    let first = app
        .save_product(Product {
            sku: "A".into(),
            name: "Producto A".into(),
            price_cents: 100,
            stock_units: 20000,
            ..Default::default()
        })
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let second = app
        .save_product(Product {
            sku: "B".into(),
            name: "Producto B".into(),
            price_cents: 100,
            ..Default::default()
        })
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let request = Checkout {
        items: vec![item(first, 10000), item(second, 10000)],
        payments: vec![payment(0, 200)],
        tendered_cents: 200,
        ..Default::default()
    };
    assert_eq!(
        app.checkout(&request).unwrap_err().code,
        "INSUFFICIENT_STOCK"
    );
    assert_eq!(
        catalog::product(app.db().unwrap(), first)
            .unwrap()
            .stock_units,
        20000
    );
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        0
    );
    assert_eq!(app.cash_overview().unwrap()["expectedCents"], 0);
    let sold = app
        .checkout(&Checkout {
            items: vec![item(first, 10000)],
            payments: vec![payment(1, 100)],
            ..Default::default()
        })
        .unwrap();
    let sid = sold["sale"]["id"].as_i64().unwrap();
    let refund = ReturnInput {
        request_id: request_id(),
        sale_id: sid,
        items: vec![],
        reason: "Cancelar compra".into(),
        method: 0,
        cancel: true,
    };
    assert_eq!(
        app.return_sale(refund.clone()).unwrap_err().code,
        "INSUFFICIENT_CASH"
    );
    assert_eq!(
        catalog::product(app.db().unwrap(), first)
            .unwrap()
            .stock_units,
        10000
    );
    assert_eq!(app.sale(sid).unwrap()["lines"][0]["returnedUnits"], 0);
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM SaleReturn", []).unwrap(),
        0
    );
    let mut card_refund = refund;
    card_refund.method = 1;
    app.return_sale(card_refund).unwrap();
    assert_eq!(
        catalog::product(app.db().unwrap(), first)
            .unwrap()
            .stock_units,
        20000
    );
    db::integrity(app.db().unwrap()).unwrap();
}
