mod common;
use calamine::{Data, Reader};
use common::*;
use donovan_core::{
    business_export::BusinessExport,
    catalog,
    contacts::{Contact, CreditPayment},
    db,
    documents::{DocumentRequest, layout::Draw, pdf},
    models::{Checkout, Payment, Product, SaleItem},
    purchasing::{PurchaseInput, PurchaseItem},
    quotes::QuoteInput,
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
                retail: false,
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

    let previous = app
        .export_business(rig.directory.path(), BusinessExport::SalesToday)
        .unwrap();
    let mut book: calamine::Xlsx<_> =
        calamine::open_workbook(previous["path"].as_str().unwrap()).unwrap();
    assert_eq!(
        book.sheet_names(),
        ["Ventas", "Partidas de ventas", "Pagos de ventas"]
    );
    for sheet in book.sheet_names() {
        assert_eq!(book.worksheet_range(&sheet).unwrap().height(), 1);
    }

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
fn exports_separate_areas_and_a_workbook_with_linked_operating_records() {
    let mut rig = rig();
    let app = &mut rig.app;
    let product_id = app
        .save_product(Product {
            sku: "M-15".into(),
            barcode: Some("00012345678901234567".into()),
            name: "Manguera reforzada".into(),
            unit: 1,
            price_cents: 11_600,
            wholesale_cents: 10_000,
            wholesale_minimum_units: Some(35_000),
            stock_units: 200_000,
            tax_basis_points: 1600,
            ..Default::default()
        })
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let customer_id = app
        .save_contact(
            false,
            Contact {
                name: "=2+2".into(),
                phone: "001234567890".into(),
                credit_limit_cents: 50_000,
                ..Default::default()
            },
        )
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let supplier_id = app
        .save_contact(
            true,
            Contact {
                name: "Distribuidora del centro".into(),
                email: "proveedor@example.test".into(),
                ..Default::default()
            },
        )
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let items = vec![SaleItem {
        product_id,
        quantity_units: 15_000,
        wholesale: false,
        retail: false,
    }];
    app.cash_open(50_000).unwrap();
    let first = app
        .checkout(&Checkout {
            items: items.clone(),
            customer_id: Some(customer_id),
            due_date: Some("2026-12-15".into()),
            payments: vec![Payment {
                method: 3,
                amount_cents: 17_400,
                reference: String::new(),
            }],
            ..Default::default()
        })
        .unwrap();
    rig.clock.advance(24 * 3600);
    app.login("admin", ADMIN_PASSWORD).unwrap();
    let today = app.now_string()[..10].to_owned();
    let second = app
        .checkout(&Checkout {
            items: items.clone(),
            payments: vec![Payment {
                method: 0,
                amount_cents: 17_400,
                reference: String::new(),
            }],
            tendered_cents: 20_000,
            ..Default::default()
        })
        .unwrap();
    app.return_sale(ReturnInput {
        request_id: request_id(),
        sale_id: second["sale"]["id"].as_i64().unwrap(),
        items: vec![ReturnItem {
            sale_line_id: second["lines"][0]["id"].as_i64().unwrap(),
            quantity_units: 5_000,
        }],
        reason: "Devolución de media unidad".into(),
        method: 0,
        cancel: false,
    })
    .unwrap();
    app.save_quote(QuoteInput {
        request_id: request_id(),
        id: 0,
        items,
        customer_id: Some(customer_id),
        discount_parts: 0,
        valid_until: "2026-12-15".into(),
        notes: "Entrega a domicilio".into(),
    })
    .unwrap();
    app.receive_purchase(PurchaseInput {
        request_id: request_id(),
        supplier_id,
        reference: "FAC-CENTRO-1".into(),
        date: today.clone(),
        paid_from_cash: false,
        items: vec![PurchaseItem {
            product_id,
            quantity_units: 30_000,
            unit_cost_cents: 6400,
            by_pack: false,
        }],
    })
    .unwrap();

    let open = |scope| {
        let result = app.export_business(rig.directory.path(), scope).unwrap();
        let path = result["path"].as_str().unwrap();
        let book: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
        (path.to_owned(), book)
    };
    let (filename, mut daily) = open(BusinessExport::SalesToday);
    assert!(filename.contains(&today));
    let folio_today = second["sale"]["folio"].as_str().unwrap();
    for tab in daily.sheet_names() {
        let range = daily.worksheet_range(&tab).unwrap();
        assert_eq!(
            range.height(),
            2,
            "{tab} debe contener solo la venta de hoy"
        );
        assert_eq!(range[(1, 0)], Data::String(folio_today.into()));
        assert!(matches!(range[(1, 1)], Data::DateTime(_)));
    }
    let (_, mut sales) = open(BusinessExport::Sales);
    let sales_tab = sales.worksheet_range("Ventas").unwrap();
    assert_eq!(sales_tab.height(), 3);
    assert_eq!(
        sales_tab[(1, 0)],
        Data::String(first["sale"]["folio"].as_str().unwrap().into())
    );
    assert_eq!(sales_tab[(2, 9)], Data::Float(174.0));
    assert_eq!(sales_tab[(2, 10)], Data::Float(58.0));

    let (_, mut customers) = open(BusinessExport::Customers);
    assert_eq!(customers.sheet_names(), ["Clientes"]);
    let directory = customers.worksheet_range("Clientes").unwrap();
    assert_eq!(directory[(1, 0)], Data::String("=2+2".into()));
    assert_eq!(directory[(1, 1)], Data::String("001234567890".into()));
    assert_eq!(directory[(1, 5)], Data::Float(174.0));
    assert!(customers.worksheet_formula("Clientes").unwrap().is_empty());
    for (scope, first_tab) in [
        (BusinessExport::Suppliers, "Proveedores"),
        (BusinessExport::Inventory, "Productos"),
        (BusinessExport::Purchases, "Compras"),
        (BusinessExport::Quotes, "Cotizaciones"),
        (BusinessExport::Returns, "Devoluciones"),
        (BusinessExport::Cash, "Caja"),
        (BusinessExport::Credit, "Movimientos de crédito"),
    ] {
        let (_, mut book) = open(scope);
        assert_eq!(book.sheet_names()[0], first_tab);
        assert!(book.worksheet_range(first_tab).unwrap().height() > 1);
    }

    let (filename, mut complete) = open(BusinessExport::All);
    assert_eq!(complete.sheet_names().len(), 16);
    for tab in complete.sheet_names() {
        assert!(
            complete.worksheet_range(&tab).unwrap().height() > 1,
            "{tab}"
        );
    }
    let catalog = complete.worksheet_range("Productos").unwrap();
    assert_eq!(catalog[(1, 1)], Data::String("00012345678901234567".into()));
    assert_eq!(catalog[(1, 12)], Data::Float(3.5));
    assert_eq!(
        complete.worksheet_range("Partidas de ventas").unwrap()[(1, 6)],
        Data::Float(1.5)
    );
    assert_eq!(
        complete.worksheet_range("Partidas de compras").unwrap()[(1, 10)],
        Data::Float(192.0)
    );
    assert_eq!(complete.worksheet_range("Ventas").unwrap(), sales_tab);
    fixture("base-del-negocio.xlsx", &std::fs::read(filename).unwrap());

    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(
        app.export_business(rig.directory.path(), BusinessExport::All)
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
            retail: false,
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
