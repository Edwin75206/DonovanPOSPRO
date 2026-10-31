mod common;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use common::*;
use donovan_core::{
    business::BusinessInput,
    contacts::Contact,
    db,
    documents::{DocumentRequest, layout::Draw, pdf},
    models::{Checkout, Payment, Product, SaleItem},
    purchasing::{PurchaseInput, PurchaseItem},
    quotes::QuoteInput,
};
use std::{fs::OpenOptions, io::Write, path::PathBuf};

fn save_fixture(name: &str, bytes: &[u8]) {
    if let Ok(folder) = std::env::var("DONOVAN_DOCUMENT_QA_OUTPUT") {
        let path = PathBuf::from(folder).join(name);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }
}
#[test]
fn branded_documents_embed_fonts_preserve_amounts_paginate_and_respect_roles() {
    let mut rig = rig();
    let app = &mut rig.app;
    app.save_business(BusinessInput {
        store_name: "Construrama Chicontla".into(),
        subtitle: "Materiales, renta de maquinaria y acarreos".into(),
        address: "Chicontla, Puebla · Domicilio de prueba para revisión de impresión".into(),
        phone: "000 000 0000".into(),
        email: "".into(),
        tax_id: "XAXX010101000".into(),
        ticket_footer:
            "Gracias por construir con nosotros.\nConserva tu ticket para cualquier aclaración."
                .into(),
        folio_prefix: "V".into(),
        default_tax_basis_points: 1600,
        ticket_width_mm: 80,
        printer_name: "".into(),
        auto_print: false,
        logo_action: "replace".into(),
        logo_data: Some(STANDARD.encode(include_bytes!(
            "../../../assets/branding/customer-logo-transparent.png"
        ))),
    })
    .unwrap();
    app.cash_open(100_000).unwrap();
    let product = app
        .save_product(Product {
            sku: "CABLE-12-ROJO".into(),
            barcode: Some("7501234567893".into()),
            name: "Cable eléctrico calibre 12 rojo · cobre 100%".into(),
            unit: 1,
            price_cents: 1850,
            cost_cents: Some(1000),
            stock_units: 100_000,
            minimum_units: 10_000,
            tax_basis_points: 1600,
            ..Default::default()
        })
        .unwrap();
    let pid = product["id"].as_i64().unwrap();
    let item = SaleItem {
        product_id: pid,
        quantity_units: 25000,
        wholesale: false,
    };
    let sale = app
        .checkout(&Checkout {
            items: vec![item.clone()],
            payments: vec![Payment {
                method: 0,
                amount_cents: 4625,
                reference: "".into(),
            }],
            tendered_cents: 5000,
            ..Default::default()
        })
        .unwrap();
    let sale_id = sale["sale"]["id"].as_i64().unwrap();
    let quote=app.save_quote(QuoteInput{request_id:request_id(),id:0,items:vec![item.clone()],customer_id:None,discount_parts:500,valid_until:"2026-11-15".into(),notes:"Incluye el precio acordado al cotizar. Entrega sujeta a disponibilidad; confirma cantidades y medidas antes de comprar.".into()}).unwrap();
    let supplier = app
        .save_contact(
            true,
            Contact {
                name: "Proveedor de materiales de prueba".into(),
                ..Default::default()
            },
        )
        .unwrap();
    let purchase = app
        .receive_purchase(PurchaseInput {
            request_id: request_id(),
            supplier_id: supplier["id"].as_i64().unwrap(),
            reference: "FACT-100".into(),
            date: "2026-10-31".into(),
            paid_from_cash: false,
            items: vec![PurchaseItem {
                product_id: pid,
                quantity_units: 50000,
                unit_cost_cents: 1000,
                by_pack: false,
            }],
        })
        .unwrap();
    let cases = [
        ("ticket-58mm.pdf", "sale", sale_id, 58),
        ("ticket-80mm.pdf", "sale", sale_id, 80),
        (
            "cotizacion-A4.pdf",
            "quote",
            quote["quote"]["id"].as_i64().unwrap(),
            80,
        ),
        (
            "compra-A4.pdf",
            "purchase",
            purchase["id"].as_i64().unwrap(),
            80,
        ),
    ];
    for (file, kind, id, width) in cases {
        let document = app
            .business_document(&DocumentRequest {
                kind: kind.into(),
                id,
                width_mm: Some(width),
                ..Default::default()
            })
            .unwrap();
        assert!(!document.pages.is_empty());
        assert!(document.logo_data.is_some());
        assert!(document.warnings.is_empty(), "{:?}", document.warnings);
        for page in &document.pages {
            for op in &page.operations {
                if let Draw::Text { x, y, width, .. } = op {
                    assert!(*x >= page.margin - 0.1);
                    assert!(
                        *x + *width <= page.width - page.margin + 0.2,
                        "{file}: text clipped at {}",
                        x + width
                    );
                    assert!(*y < page.height - 1.0);
                }
            }
        }
        let bytes = pdf::encode(&document).unwrap();
        let parsed = lopdf::Document::load_mem(&bytes).unwrap();
        let page_numbers: Vec<u32> = parsed.get_pages().keys().copied().collect();
        let text = parsed.extract_text(&page_numbers).unwrap();
        assert!(text.contains("Chicontla"));
        assert!(text.contains("eléctrico"));
        if kind == "sale" {
            assert!(text.contains("$46.25"));
            assert!(text.contains("$3.75"));
            assert!(text.contains("2.5"));
            assert!(text.contains("MXN"));
        }
        assert!(
            parsed
                .objects
                .values()
                .any(|v| v.as_dict().is_ok_and(|d| d.has(b"FontFile2")))
        );
        save_fixture(file, &bytes);
        #[cfg(windows)]
        if kind == "quote" && std::env::var("DONOVAN_DOCUMENT_QA_PRINT").as_deref() == Ok("1") {
            let folder = PathBuf::from(std::env::var("DONOVAN_DOCUMENT_QA_OUTPUT").unwrap());
            let output = folder.join("cotizacion-Windows-GDI.pdf");
            let job = donovan_core::documents::printing::print(
                &document,
                "Microsoft Print to PDF",
                Some(&output),
            )
            .unwrap();
            assert!(job.job_id > 0);
            assert!(output.exists());
            let printed = lopdf::Document::load(&output).unwrap();
            assert_eq!(printed.get_pages().len(), document.pages.len());
        }
    }
    let mut long_items = vec![];
    for index in 0..35 {
        let p=app.save_product(Product{sku:format!("PRODUCTO-{index:03}"),name:format!("Material de construcción número {index}: descripción extensa para comprobar el salto de página del comprobante"),price_cents:10_000,stock_units:100_000,tax_basis_points:1600,..Default::default()}).unwrap();
        long_items.push(SaleItem {
            product_id: p["id"].as_i64().unwrap(),
            quantity_units: 10000,
            wholesale: false,
        });
    }
    let long_sale = app
        .checkout(&Checkout {
            items: long_items,
            payments: vec![Payment {
                method: 0,
                amount_cents: 350_000,
                reference: "".into(),
            }],
            tendered_cents: 350_000,
            ..Default::default()
        })
        .unwrap();
    let document = app
        .business_document(&DocumentRequest {
            kind: "sale".into(),
            id: long_sale["sale"]["id"].as_i64().unwrap(),
            width_mm: Some(58),
            ..Default::default()
        })
        .unwrap();
    assert!(document.pages.len() > 2);
    save_fixture("ticket-largo-58mm.pdf", &pdf::encode(&document).unwrap());
    let before = db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap();
    let folder = tempfile::tempdir().unwrap();
    let exported = app
        .export_document(
            &DocumentRequest {
                kind: "sale".into(),
                id: sale_id,
                width_mm: Some(80),
                ..Default::default()
            },
            folder.path(),
        )
        .unwrap();
    assert!(PathBuf::from(exported["path"].as_str().unwrap()).exists());
    assert_eq!(
        db::scalar(app.db().unwrap(), "SELECT COUNT(*) FROM Sale", []).unwrap(),
        before
    );
    app.logout().unwrap();
    app.login("negociante.uno", SELLER_PASSWORD).unwrap();
    assert_eq!(
        app.business_document(&DocumentRequest {
            kind: "sale".into(),
            id: sale_id,
            width_mm: Some(80),
            ..Default::default()
        })
        .unwrap_err()
        .code,
        "FORBIDDEN"
    );
    db::integrity(app.db().unwrap()).unwrap();
}
