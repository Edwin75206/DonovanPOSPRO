mod common;
use calamine::{Data, Reader};
use common::*;
use donovan_core::{
    catalog,
    contacts::{Contact, CreditPayment},
    db,
    documents::{DocumentRequest, layout::Draw, pdf},
    models::{Checkout, Payment, Product, SaleItem},
    purchasing::{PurchaseInput, PurchaseItem},
    reports::ReportQuery,
    returns::{ReturnInput, ReturnItem},
};
use std::{fs::OpenOptions, io::Write, path::PathBuf};

fn query(kind: &str, from: &str, to: &str) -> ReportQuery {
    ReportQuery {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        search: String::new(),
    }
}
fn fixture(name: &str, bytes: &[u8]) {
    if let Ok(folder) = std::env::var("DONOVAN_DOCUMENT_QA_OUTPUT") {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(PathBuf::from(folder).join(name))
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }
}
fn assert_paper(document: &donovan_core::documents::layout::BusinessDocument) {
    for page in &document.pages {
        for op in &page.operations {
            match op {
                Draw::Text {
                    x, y, width, text, ..
                } => assert!(
                    *x >= 0.0 && *y >= 0.0 && *x + *width <= page.width + 0.5 && *y <= page.height,
                    "Text outside paper: {text}"
                ),
                Draw::Rect {
                    x,
                    y,
                    width,
                    height,
                    ..
                }
                | Draw::Logo {
                    x,
                    y,
                    width,
                    height,
                } => assert!(
                    *x >= 0.0
                        && *y >= 0.0
                        && *x + *width <= page.width + 0.5
                        && *y + *height <= page.height + 0.5
                ),
            }
        }
    }
}
#[test]
fn reports_reconcile_event_dates_credit_returns_cash_and_exact_purchase_costs() {
    let mut rig = rig();
    let app = &mut rig.app;
    let day1 = app.now_string()[..10].to_owned();
    app.cash_open(100_000).unwrap();
    let product = app
        .save_product(Product {
            sku: "MART-16".into(),
            barcode: Some("0012345678901".into()),
            name: "Martillo de uña 16 oz · mango reforzado".into(),
            category: "Herramientas".into(),
            price_cents: 11600,
            cost_cents: Some(6000),
            stock_units: 100000,
            tax_basis_points: 1600,
            ..Default::default()
        })
        .unwrap();
    let pid = product["id"].as_i64().unwrap();
    let customer = app
        .save_contact(
            false,
            Contact {
                name: "=2+2".into(),
                credit_limit_cents: 50000,
                ..Default::default()
            },
        )
        .unwrap();
    let cid = customer["id"].as_i64().unwrap();
    let sale = app
        .checkout(&Checkout {
            items: vec![SaleItem {
                product_id: pid,
                quantity_units: 20000,
                wholesale: false,
            }],
            payments: vec![
                Payment {
                    method: 0,
                    amount_cents: 10000,
                    reference: String::new(),
                },
                Payment {
                    method: 3,
                    amount_cents: 13200,
                    reference: String::new(),
                },
            ],
            tendered_cents: 10000,
            customer_id: Some(cid),
            due_date: Some("2026-12-31".into()),
            ..Default::default()
        })
        .unwrap();
    let sid = sale["sale"]["id"].as_i64().unwrap();
    let line = sale["lines"][0]["id"].as_i64().unwrap();
    let mut current = catalog::product(app.db().unwrap(), pid).unwrap();
    current.cost_cents = Some(9000);
    app.save_product(current).unwrap();
    rig.clock.advance(22 * 3600);
    let day2 = app.now_string()[..10].to_owned();
    assert_ne!(day1, day2);
    app.return_sale(ReturnInput {
        request_id: request_id(),
        sale_id: sid,
        items: vec![ReturnItem {
            sale_line_id: line,
            quantity_units: 10000,
        }],
        reason: "Revisión de devolución en fecha posterior".into(),
        method: 0,
        cancel: false,
    })
    .unwrap();
    app.credit_payment(CreditPayment {
        request_id: request_id(),
        customer_id: cid,
        sale_id: sid,
        amount_cents: 800,
        method: 1,
        reference: "Abono de prueba".into(),
    })
    .unwrap();
    app.cash_movement(200, "Ingreso de prueba").unwrap();
    app.cash_movement(-150, "Egreso de prueba").unwrap();
    app.cash_close(110000, "Cierre de prueba").unwrap();
    let pack = app
        .save_product(Product {
            sku: "PACK-3".into(),
            name: "Empaque de tres unidades".into(),
            price_cents: 5000,
            pack_units: 30000,
            ..Default::default()
        })
        .unwrap();
    let supplier = app
        .save_contact(
            true,
            Contact {
                name: "Proveedor de prueba".into(),
                ..Default::default()
            },
        )
        .unwrap();
    app.receive_purchase(PurchaseInput {
        request_id: request_id(),
        supplier_id: supplier["id"].as_i64().unwrap(),
        reference: "PACK-REDONDEO".into(),
        date: day2.clone(),
        paid_from_cash: false,
        items: vec![PurchaseItem {
            product_id: pack["id"].as_i64().unwrap(),
            quantity_units: 10000,
            unit_cost_cents: 10000,
            by_pack: true,
        }],
    })
    .unwrap();

    let daily = app.report(&query("daily", &day1, &day2)).unwrap();
    assert_eq!(daily.rows.len(), 2);
    assert_eq!(daily.rows[0]["netCents"], 23200);
    assert_eq!(daily.rows[1]["netCents"], -11600);
    let dash = app.dashboard(&day1, &day2).unwrap();
    assert_eq!(dash["totalCents"], 11600);
    assert_eq!(daily.metrics[2].value, Some(11600));
    for kind in ["products", "categories", "users"] {
        let report = app.report(&query(kind, &day1, &day2)).unwrap();
        assert_eq!(report.rows[0]["netCents"], 11600);
        assert_eq!(report.metrics[2].value, Some(11600));
    }
    let profit = app.report(&query("profit", &day1, &day2)).unwrap();
    assert_eq!(profit.rows[0]["netCents"], 10000);
    assert_eq!(profit.rows[0]["costCents"], 6000);
    assert_eq!(profit.rows[0]["profitCents"], 4000);
    assert_eq!(dash["profitCents"], 4000);
    assert_eq!(
        app.report(&query("profit", &day2, &day2)).unwrap().rows[0]["profitCents"],
        -4000
    );
    let methods = app.report(&query("methods", &day2, &day2)).unwrap();
    assert_eq!(methods.metrics[2].value, Some(800));
    assert_eq!(
        app.report(&query("receivables", &day1, &day2))
            .unwrap()
            .rows[0]["balanceCents"],
        800
    );
    assert_eq!(
        app.report(&query("credits", &day2, &day2)).unwrap().metrics[1].value,
        Some(-12400)
    );
    let returned = app.report(&query("returns", &day2, &day2)).unwrap();
    assert_eq!(returned.rows[0]["totalCents"], 11600);
    assert_eq!(returned.rows[0]["creditReductionCents"], 11600);
    assert_eq!(returned.rows[0]["paidCents"], 0);
    assert_eq!(
        app.report(&query("cash", &day1, &day2)).unwrap().rows[0]["differenceCents"],
        -50
    );
    assert_eq!(
        app.report(&query("cashMovements", &day2, &day2))
            .unwrap()
            .metrics[1]
            .value,
        Some(50)
    );
    assert_eq!(
        app.report(&query("purchases", &day1, &day2)).unwrap().rows[0]["totalCents"],
        10000
    );
    let mut search = query("products", &day1, &day2);
    search.search = "UNA".into();
    assert_eq!(app.report(&search).unwrap().rows.len(), 1);
    search.search = "No coincide".into();
    assert!(app.report(&search).unwrap().rows.is_empty());

    let export = tempfile::TempDir::new().unwrap();
    for kind in [
        "daily",
        "sales",
        "products",
        "categories",
        "users",
        "methods",
        "profit",
        "inventory",
        "purchases",
        "receivables",
        "credits",
        "cash",
        "cashMovements",
        "returns",
    ] {
        let input = query(kind, &day1, &day2);
        let report = app.report(&input).unwrap();
        for row in &report.rows {
            for column in &report.columns {
                assert!(
                    row.get(&column.key).is_some(),
                    "{kind}: missing {}",
                    column.key
                );
            }
        }
        let document = app
            .business_document(&DocumentRequest {
                kind: "report".into(),
                report: Some(input.clone()),
                ..Default::default()
            })
            .unwrap();
        assert_paper(&document);
        assert!(document.pages[0].width > document.pages[0].height);
        let bytes = pdf::encode(&document).unwrap();
        let decoded = lopdf::Document::load_mem(&bytes).unwrap();
        assert_eq!(decoded.get_pages().len(), document.pages.len());
        fixture(&format!("reporte-{kind}.pdf"), &bytes);
        let excel = app.export_report(export.path(), &input).unwrap();
        let path = excel["path"].as_str().unwrap();
        let mut workbook = calamine::open_workbook_auto(path).unwrap();
        let range = workbook.worksheet_range("Detalle").unwrap();
        assert_eq!(range.height(), report.rows.len() + 1);
        assert_eq!(range.width(), report.columns.len());
        assert!(
            workbook
                .worksheet_formula("Detalle")
                .unwrap()
                .cells()
                .all(|(_, _, formula)| formula.is_empty())
        );
        for (r, row) in report.rows.iter().enumerate() {
            for (c, column) in report.columns.iter().enumerate() {
                if let Some(cents) = row[&column.key].as_i64().filter(|_| column.kind == "money") {
                    assert!(
                        matches!(&range[(r+1,c)],Data::Float(n) if ((*n*100.0).round()as i64)==cents),
                        "{kind}: numeric money did not round trip"
                    );
                }
                if row[&column.key] == "=2+2" {
                    assert_eq!(range[(r + 1, c)], Data::String("=2+2".into()));
                }
            }
        }
        fixture(
            &format!("reporte-{kind}.xlsx"),
            &std::fs::read(path).unwrap(),
        );
        #[cfg(windows)]
        if kind == "inventory" && std::env::var("DONOVAN_DOCUMENT_QA_PRINT").as_deref() == Ok("1") {
            let path = PathBuf::from(std::env::var("DONOVAN_DOCUMENT_QA_OUTPUT").unwrap())
                .join("reporte-inventario-Windows-GDI.pdf");
            let job = donovan_core::documents::printing::print(
                &document,
                "Microsoft Print to PDF",
                Some(&path),
            )
            .unwrap();
            assert!(job.job_id > 0);
            let printed = lopdf::Document::load(path).unwrap();
            assert_eq!(printed.get_pages().len(), document.pages.len());
        }
    }
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        1
    );
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM SaleReturn", []).unwrap(),
        1
    );
    app.logout().unwrap();
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(
        app.report(&query("profit", &day1, &day2)).unwrap_err().code,
        "FORBIDDEN"
    );
    assert_eq!(
        app.business_document(&DocumentRequest {
            kind: "report".into(),
            report: Some(query("profit", &day1, &day2)),
            ..Default::default()
        })
        .unwrap_err()
        .code,
        "FORBIDDEN"
    );
}

#[test]
fn missing_cost_is_never_reported_as_zero_and_labels_preserve_codes() {
    let mut rig = rig();
    let app = &mut rig.app;
    let day = app.now_string()[..10].to_owned();
    app.cash_open(10000).unwrap();
    let product=app.save_product(Product{sku:"CABLE-12-ROJO".into(),barcode:Some("0012345678901".into()),name:"Cable eléctrico de cobre rojo calibre 12, descripción extensa para revisar el espacio y la lectura de la etiqueta".into(),price_cents:1850,stock_units:100000,unit:1,..Default::default()}).unwrap();
    let pid = product["id"].as_i64().unwrap();
    app.checkout(&Checkout {
        items: vec![SaleItem {
            product_id: pid,
            quantity_units: 10000,
            wholesale: false,
        }],
        payments: vec![Payment {
            method: 0,
            amount_cents: 1850,
            reference: String::new(),
        }],
        tendered_cents: 1850,
        ..Default::default()
    })
    .unwrap();
    let report = app.report(&query("profit", &day, &day)).unwrap();
    assert!(report.rows[0]["costCents"].is_null());
    assert!(report.rows[0]["profitCents"].is_null());
    assert_eq!(report.metrics[2].value, None);
    let mut product = catalog::product(app.db().unwrap(), pid).unwrap();
    for (barcode, name) in [
        (Some("0012345678901"), "etiquetas-numericas"),
        (None, "etiquetas-alfanumericas"),
    ] {
        product.barcode = barcode.map(str::to_owned);
        app.save_product(product.clone()).unwrap();
        let document = app
            .business_document(&DocumentRequest {
                kind: "labels".into(),
                id: pid,
                copies: Some(17),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(document.pages.len(), 2);
        assert_paper(&document);
        let bytes = pdf::encode(&document).unwrap();
        let pdf = lopdf::Document::load_mem(&bytes).unwrap();
        assert!(
            pdf.extract_text(&[1])
                .unwrap()
                .contains(barcode.unwrap_or("CABLE-12-ROJO"))
        );
        fixture(&format!("{name}.pdf"), &bytes);
        #[cfg(windows)]
        if barcode.is_some() && std::env::var("DONOVAN_DOCUMENT_QA_PRINT").as_deref() == Ok("1") {
            let path = PathBuf::from(std::env::var("DONOVAN_DOCUMENT_QA_OUTPUT").unwrap())
                .join("etiquetas-Windows-GDI.pdf");
            let job = donovan_core::documents::printing::print(
                &document,
                "Microsoft Print to PDF",
                Some(&path),
            )
            .unwrap();
            assert!(job.job_id > 0);
            assert_eq!(lopdf::Document::load(path).unwrap().get_pages().len(), 2);
        }
    }
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        1
    );
    product.barcode = Some("X".repeat(64));
    app.save_product(product.clone()).unwrap();
    assert_eq!(
        app.business_document(&DocumentRequest {
            kind: "labels".into(),
            id: pid,
            ..Default::default()
        })
        .unwrap_err()
        .code,
        "LABEL_CODE"
    );
    product.barcode = Some("MÉTRICO".into());
    app.save_product(product).unwrap();
    assert_eq!(
        app.business_document(&DocumentRequest {
            kind: "labels".into(),
            id: pid,
            ..Default::default()
        })
        .unwrap_err()
        .code,
        "LABEL_CODE"
    );
    assert_eq!(
        app.business_document(&DocumentRequest {
            kind: "labels".into(),
            id: pid,
            copies: Some(501),
            ..Default::default()
        })
        .unwrap_err()
        .code,
        "VALIDATION"
    );
    app.logout().unwrap();
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(
        app.business_document(&DocumentRequest {
            kind: "labels".into(),
            id: pid,
            ..Default::default()
        })
        .unwrap_err()
        .code,
        "FORBIDDEN"
    );
}
