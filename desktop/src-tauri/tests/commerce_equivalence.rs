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
fn quote_revision_preserves_folio_and_inventory_and_cannot_rewrite_a_sale() {
    let mut rig = rig();
    let app = &mut rig.app;
    let product = app
        .save_product(Product {
            sku: "REVISION".into(),
            name: "Material de prueba".into(),
            price_cents: 100,
            stock_units: 100000,
            ..Default::default()
        })
        .unwrap();
    let pid = product["id"].as_i64().unwrap();
    let mut input = QuoteInput {
        request_id: request_id(),
        id: 0,
        items: vec![item(pid, 10000)],
        customer_id: None,
        discount_parts: 0,
        valid_until: "2026-11-15".into(),
        notes: "Condición original".into(),
    };
    let original = app.save_quote(input.clone()).unwrap();
    input.id = original["quote"]["id"].as_i64().unwrap();
    input.request_id = request_id();
    input.items = vec![item(pid, 20000)];
    input.valid_until = "2026-12-01".into();
    input.notes = "Entrega acordada".into();
    let mut current = catalog::product(app.db().unwrap(), pid).unwrap();
    current.price_cents = 250;
    app.save_product(current).unwrap();
    // Viewing or changing the catalog never rewrites the agreed proposal.
    assert_eq!(app.quote(input.id).unwrap(), original);
    let revised = app.save_quote(input.clone()).unwrap();
    assert_eq!(revised["quote"]["folio"], original["quote"]["folio"]);
    assert_eq!(revised["quote"]["totalCents"], 500);
    assert_eq!(revised["quote"]["validUntil"], "2026-12-01");
    assert_eq!(revised["quote"]["notes"], "Entrega acordada");
    assert_eq!(app.save_quote(input.clone()).unwrap(), revised);
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Quote", []).unwrap(),
        1
    );
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        100000
    );
    input.request_id = request_id();
    input.items = vec![item(pid, -10000)];
    assert!(app.save_quote(input.clone()).is_err());
    assert_eq!(app.quote(input.id).unwrap(), revised);
    app.cash_open(0).unwrap();
    let sale = app
        .checkout(&Checkout {
            quote_id: Some(input.id),
            payments: vec![payment(0, 500)],
            tendered_cents: 500,
            ..Default::default()
        })
        .unwrap();
    input.request_id = request_id();
    input.items = vec![item(pid, 10000)];
    assert_eq!(
        app.save_quote(input.clone()).unwrap_err().code,
        "ALREADY_COMPLETED"
    );
    let preserved = app.quote(input.id).unwrap();
    assert_eq!(preserved["quote"]["totalCents"], 500);
    assert_eq!(preserved["quote"]["saleId"], sale["sale"]["id"]);
    assert_eq!(
        catalog::product(app.db().unwrap(), pid)
            .unwrap()
            .stock_units,
        80000
    );
    db::integrity(app.db().unwrap()).unwrap();
}
#[test]
fn cash_retries_do_not_duplicate_money_or_close_a_later_session() {
    let mut rig = rig();
    let app = &mut rig.app;
    let open = request_id();
    let first = app.cash_open_once(&open, 10000).unwrap()["open"]["id"].clone();
    app.cash_open_once(&open, 10000).unwrap();
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM CashSession", []).unwrap(),
        1
    );
    let movement = request_id();
    app.cash_movement_once(&movement, 2000, "Ingreso de prueba")
        .unwrap();
    assert_eq!(
        app.cash_movement_once(&movement, 2000, "Ingreso de prueba")
            .unwrap()["expectedCents"],
        12000
    );
    assert_eq!(
        app.cash_movement_once(&movement, 3000, "Ingreso de prueba")
            .unwrap_err()
            .code,
        "REQUEST_CHANGED"
    );
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM CashMovement", []).unwrap(),
        1
    );
    let closing = request_id();
    app.cash_close_once(&closing, 12000, "Revisión").unwrap();
    let second = app.cash_open_once(&request_id(), 5000).unwrap()["open"]["id"].clone();
    assert_ne!(first, second);
    assert_eq!(
        app.cash_close_once(&closing, 12000, "Revisión").unwrap()["open"]["id"],
        second
    );
    assert_eq!(
        app.cash_movement_once(&movement, 2000, "Ingreso de prueba")
            .unwrap()["expectedCents"],
        5000
    );
    assert_eq!(
        app.cash_open_once(&open, 10000).unwrap()["open"]["id"],
        second
    );
    db::integrity(app.db().unwrap()).unwrap();
}
#[test]
fn sales_history_sorts_before_paging_and_keeps_other_users_sales_private() {
    let mut rig = rig();
    let app = &mut rig.app;
    app.cash_open(10000).unwrap();
    let product = app
        .save_product(Product {
            sku: "SORT-REPORT".into(),
            name: "Material para comprobar paginación".into(),
            price_cents: 10,
            stock_units: 100_000_000,
            ..Default::default()
        })
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    for quantity in 1..=62 {
        app.checkout(&Checkout {
            items: vec![item(product, quantity * 10000)],
            payments: vec![payment(0, quantity * 10)],
            tendered_cents: quantity * 10,
            ..Default::default()
        })
        .unwrap();
    }
    let query = |page| donovan_core::history::SalesQuery {
        from: "2026-10-31".into(),
        to: "2026-10-31".into(),
        search: String::new(),
        method: None,
        status: "all".into(),
        page,
        sort: "amount".into(),
        descending: false,
    };
    let first = app.sales_page(query(0)).unwrap();
    assert_eq!(first["total"], 62);
    assert_eq!(first["items"][0]["totalCents"], 10);
    assert_eq!(first["items"][59]["totalCents"], 600);
    let second = app.sales_page(query(1)).unwrap();
    assert_eq!(second["items"].as_array().unwrap().len(), 2);
    assert_eq!(second["items"][0]["totalCents"], 610);
    let mut invalid = query(0);
    invalid.sort = "s.Id; DELETE FROM Sale".into();
    assert_eq!(app.sales_page(invalid).unwrap_err().code, "VALIDATION");
    app.logout().unwrap();
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(app.sales_page(query(0)).unwrap()["total"], 0);
    db::integrity(app.db().unwrap()).unwrap();
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
    let fractional_pack = app
        .save_product(Product {
            sku: "PACK-3".into(),
            name: "Presentación indivisible de costo".into(),
            price_cents: 10000,
            pack_units: 30000,
            ..Default::default()
        })
        .unwrap();
    let exact = app
        .receive_purchase(PurchaseInput {
            request_id: request_id(),
            supplier_id: supplier["id"].as_i64().unwrap(),
            reference: "FACTURA-REDONDEO".into(),
            date: "2026-10-31".into(),
            paid_from_cash: false,
            items: vec![PurchaseItem {
                product_id: fractional_pack["id"].as_i64().unwrap(),
                quantity_units: 10000,
                unit_cost_cents: 10000,
                by_pack: true,
            }],
        })
        .unwrap();
    let exact_detail = app.purchase(exact["id"].as_i64().unwrap()).unwrap();
    assert_eq!(exact_detail["purchase"]["sourceExact"], true);
    assert_eq!(exact_detail["lines"][0]["quantityUnits"], 30000);
    assert_eq!(exact_detail["lines"][0]["costCents"], 3333);
    assert_eq!(exact_detail["lines"][0]["totalCents"], 10000);
    assert_eq!(exact_detail["lines"][0]["enteredCostCents"], 10000);
    db::integrity(app.db().unwrap()).unwrap();
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
    let preview = app.price_cart(&[], 0, Some(qid)).unwrap();
    assert_eq!(preview.total_cents, 200);
    assert_eq!(preview.discount_cents, 100);
    assert_eq!(preview.lines[0].price_cents, 100);
    let request = Checkout {
        quote_id: Some(qid),
        payments: vec![payment(0, 80), payment(3, 120)],
        tendered_cents: 100,
        customer_id: Some(cid),
        due_date: Some("2026-11-15".into()),
        ..Default::default()
    };
    let sold = app.checkout(&request).unwrap();
    assert_eq!(
        app.checkout_result(&request.request_id).unwrap()["sale"]["id"],
        sold["sale"]["id"]
    );
    assert!(app.checkout_result(&request_id()).unwrap().is_null());
    assert_eq!(
        app.price_cart(&[], 0, Some(qid)).unwrap_err().code,
        "ALREADY_COMPLETED"
    );
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
        assert_eq!(
            app.return_sale_checked(refund.clone(), Some((*amount + 1, *amount)))
                .unwrap_err()
                .code,
            "RETURN_CHANGED"
        );
        assert_eq!(
            db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM SaleReturn", []).unwrap(),
            index as i64
        );
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
