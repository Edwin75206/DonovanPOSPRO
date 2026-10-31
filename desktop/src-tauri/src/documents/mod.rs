mod labels;
pub mod layout;
pub mod pdf;
#[cfg(windows)]
pub mod printing;
mod report;
use crate::{
    Engine, db,
    error::{AppError, Result, ensure},
    storage,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use layout::{BusinessDocument, GRAY, INK, Layout, money, quantity, unit};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs::OpenOptions, io::Write, path::Path};

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRequest {
    pub kind: String,
    pub id: i64,
    pub width_mm: Option<i64>,
    pub report: Option<crate::reports::ReportQuery>,
    pub copies: Option<i64>,
    pub ticket: Option<crate::tickets::TicketConfig>,
}
fn s<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
fn n(value: &Value, key: &str) -> i64 {
    value[key].as_i64().unwrap_or(0)
}
// Render a monochrome copy for thermal output. The stored original and its geometry are untouched.
pub fn thermal_logo(logo: Option<String>, monochrome: bool) -> Result<Option<String>> {
    if !monochrome {
        return Ok(logo);
    }
    let Some(source) = logo else { return Ok(None) };
    let bytes = STANDARD
        .decode(source)
        .map_err(|e| AppError::internal("DOCUMENT_IMAGE", "No pudimos leer el logotipo.", e))?;
    crate::appearance::validate_image(&bytes)?;
    let mut bitmap = image::load_from_memory(&bytes)
        .map_err(|e| AppError::internal("DOCUMENT_IMAGE", "No pudimos preparar el logotipo.", e))?
        .to_rgba8();
    for pixel in bitmap.pixels_mut() {
        let luminance =
            (2126 * pixel[0] as u32 + 7152 * pixel[1] as u32 + 722 * pixel[2] as u32) / 10000;
        let color = if luminance < 190 { 0 } else { 255 };
        pixel[0] = color;
        pixel[1] = color;
        pixel[2] = color;
    }
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(bitmap)
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| {
            AppError::internal(
                "DOCUMENT_IMAGE",
                "No pudimos preparar el logotipo térmico.",
                e,
            )
        })?;
    Ok(Some(STANDARD.encode(output.into_inner())))
}
pub(crate) fn date(value: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f")
        .map(|date| date.format("%d/%m/%Y · %H:%M").to_string())
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map(|date| date.format("%d/%m/%Y").to_string())
        })
        .unwrap_or_else(|_| value.to_owned())
}
fn branding(layout: &mut Layout, settings: &Value) -> Result<()> {
    if let Some(data) = layout.document.logo_data.as_deref() {
        let bytes = STANDARD.decode(data).map_err(|e| {
            AppError::internal(
                "DOCUMENT_IMAGE",
                "No pudimos preparar el logotipo del documento.",
                e,
            )
        })?;
        crate::appearance::validate_image(&bytes)?;
        let image = image::load_from_memory(&bytes)
            .map_err(|e| AppError::internal("DOCUMENT_IMAGE", "No pudimos leer el logotipo.", e))?;
        layout.logo(image.width() as f32 / image.height() as f32);
    }
    layout.paragraph(
        s(settings, "storeName"),
        if layout.roll { 11.0 } else { 17.0 },
        true,
        true,
        INK,
    );
    for key in ["subtitle", "address", "phone", "email"] {
        if !s(settings, key).is_empty() {
            layout.paragraph(
                s(settings, key),
                if layout.roll { 7.5 } else { 9.0 },
                false,
                true,
                GRAY,
            );
        }
    }
    if !s(settings, "taxId").is_empty() {
        layout.paragraph(
            &format!("RFC: {}", s(settings, "taxId")),
            8.0,
            false,
            true,
            GRAY,
        );
    }
    layout.rule();
    Ok(())
}
impl Engine {
    pub fn business_document(&self, request: &DocumentRequest) -> Result<BusinessDocument> {
        self.actor("sell")?;
        let mut settings = db::one(
            self.db()?,
            "SELECT s.*,COALESCE(p.Subtitle,'') AS Subtitle FROM StoreSettings s LEFT JOIN StorePresentation p ON p.StoreId=s.Id LIMIT 1",
            [],
        )?;
        let mut ticket = crate::tickets::TicketConfig::load(self.db()?)?;
        if let Some(override_config) = &request.ticket {
            self.actor("admin")?;
            override_config.validate()?;
            ticket = override_config.clone();
        }
        let width = request.width_mm.unwrap_or(ticket.width_mm);
        settings["ticketFooter"] = json!(ticket.footer);
        ensure(
            [58, 80].contains(&width),
            "VALIDATION",
            "Selecciona un ancho de ticket de 58 u 80 mm.",
        )?;
        let logo = settings["logoData"].as_str().map(str::to_owned);
        match request.kind.as_str() {
            "report" => {
                let query = request.report.as_ref().ok_or_else(|| {
                    AppError::rule("VALIDATION", "Selecciona un reporte y sus fechas.")
                })?;
                let report = self.report(query)?;
                report::build(&report, &settings)
            }
            "labels" => {
                self.actor("catalog")?;
                labels::build(
                    &crate::catalog::product(self.db()?, request.id)?,
                    &settings,
                    request.copies.unwrap_or(16),
                )
            }
            "sale" | "ticketTest" => {
                let detail = if request.kind == "ticketTest" {
                    let actor = self.actor("admin")?;
                    json!({"sale":{"folio":"PRUEBA · NO ES UNA VENTA","cashier":actor.name,"createdAt":self.now_string(),"customerName":"Cliente de prueba","totalCents":17050,"taxCents":2352,"discountCents":0,"tenderedCents":20000,"changeCents":2950},"lines":[{"sku":"MUESTRA-01","name":"Producto de prueba","quantityUnits":20000,"unit":0,"priceCents":7500,"totalCents":15000},{"sku":"MUESTRA-02","name":"Material por metro","quantityUnits":25000,"unit":1,"priceCents":820,"totalCents":2050}],"payments":[{"method":0,"amountCents":17050}],"returns":[]})
                } else {
                    self.sale(request.id)?
                };
                let sale = &detail["sale"];
                let ticket_logo = if ticket.show_logo {
                    thermal_logo(logo, ticket.monochrome)?
                } else {
                    None
                };
                let mut paper =
                    Layout::new(s(sale, "folio"), "sale", width as f32, true, ticket_logo)?;
                paper.set_roll_margin(ticket.margin_mm as f32);
                branding(&mut paper, &settings)?;
                paper.paragraph("COMPROBANTE DE VENTA", 9.0, true, true, INK);
                paper.paragraph(s(sale, "folio"), 13.0, true, true, INK);
                if sale["cancelled"] == true {
                    paper.paragraph("CANCELADA / DEVUELTA", 9.0, true, true, INK);
                }
                paper.paragraph(&date(s(sale, "createdAt")), 7.5, false, true, GRAY);
                paper.space(5.0);
                paper.paragraph(
                    &format!("Atendió: {}", s(sale, "cashier")),
                    8.0,
                    false,
                    false,
                    INK,
                );
                paper.paragraph(
                    &format!("Cliente: {}", s(sale, "customerName")),
                    8.0,
                    false,
                    false,
                    INK,
                );
                paper.rule();
                for line in detail["lines"].as_array().into_iter().flatten() {
                    let wrapped =
                        paper
                            .fonts
                            .wrap(s(line, "name"), paper.content_width(), 9.0, true);
                    paper.ensure(wrapped.len() as f32 * 13.1 + 34.0);
                    paper.paragraph(s(line, "name"), 9.0, true, false, INK);
                    paper.paragraph(s(line, "sku"), 7.0, false, false, GRAY);
                    paper.pair(
                        &format!(
                            "{} {} × {}",
                            quantity(n(line, "quantityUnits")),
                            unit(n(line, "unit")),
                            money(n(line, "priceCents"))
                        ),
                        &money(n(line, "totalCents")),
                        8.0,
                        false,
                    );
                    let line_discount =
                        crate::money::line_total(n(line, "priceCents"), n(line, "quantityUnits"))?
                            - n(line, "totalCents");
                    if line_discount > 0 {
                        paper.paragraph(
                            &format!("Incluye descuento de {}", money(line_discount)),
                            7.0,
                            false,
                            false,
                            GRAY,
                        );
                    }
                    if n(line, "returnedUnits") > 0 {
                        paper.paragraph(
                            &format!(
                                "Devuelto: {} {}",
                                quantity(n(line, "returnedUnits")),
                                unit(n(line, "unit"))
                            ),
                            7.0,
                            false,
                            false,
                            GRAY,
                        );
                    }
                    paper.space(7.0);
                }
                paper.rule();
                paper.ensure(105.0);
                paper.pair(
                    "Subtotal sin IVA",
                    &money(n(sale, "totalCents") - n(sale, "taxCents")),
                    8.0,
                    false,
                );
                paper.pair("IVA incluido", &money(n(sale, "taxCents")), 8.0, false);
                if n(sale, "discountCents") > 0 {
                    paper.pair(
                        "Descuento ya aplicado",
                        &money(n(sale, "discountCents")),
                        8.0,
                        false,
                    );
                }
                paper.space(6.0);
                paper.pair("TOTAL", &money(n(sale, "totalCents")), 14.0, true);
                paper.paragraph("Pesos mexicanos · MXN", 7.0, false, false, GRAY);
                paper.rule();
                for payment in detail["payments"].as_array().into_iter().flatten() {
                    let method = ["Efectivo", "Tarjeta", "Transferencia", "Crédito"]
                        .get(n(payment, "method") as usize)
                        .copied()
                        .unwrap_or("Pago");
                    paper.pair(method, &money(n(payment, "amountCents")), 8.0, false);
                    if !s(payment, "reference").is_empty() {
                        paper.paragraph(
                            &format!("Ref. {}", s(payment, "reference")),
                            7.0,
                            false,
                            false,
                            GRAY,
                        );
                    }
                }
                if n(sale, "tenderedCents") > 0 {
                    paper.pair(
                        "Efectivo recibido",
                        &money(n(sale, "tenderedCents")),
                        8.0,
                        false,
                    );
                    paper.pair("Cambio", &money(n(sale, "changeCents")), 10.0, true);
                }
                if n(&detail, "creditBalanceCents") > 0 {
                    paper.pair(
                        "Crédito pendiente",
                        &money(n(&detail, "creditBalanceCents")),
                        8.0,
                        true,
                    );
                }
                let returns = detail["returns"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|r| n(r, "totalCents"))
                    .sum::<i64>();
                if returns > 0 {
                    paper.rule();
                    paper.pair("Devuelto a la fecha", &money(returns), 8.0, true);
                    paper.pair(
                        "Importe vigente",
                        &money(n(sale, "totalCents") - returns),
                        9.0,
                        true,
                    );
                }
                paper.rule();
                if !s(&settings, "ticketFooter").is_empty() {
                    paper.paragraph(s(&settings, "ticketFooter"), 7.5, false, true, INK);
                }
                paper.space(7.0);
                if !ticket.legend.is_empty() {
                    paper.paragraph(&ticket.legend, 7.0, false, true, GRAY);
                }
                paper.paragraph("No es un CFDI.", 7.0, false, true, GRAY);
                paper.finish()
            }
            "quote" => {
                let detail = self.quote(request.id)?;
                let quote = &detail["quote"];
                let mut paper = Layout::new(s(quote, "folio"), "quote", 210.0, false, logo)?;
                branding(&mut paper, &settings)?;
                paper.paragraph(
                    &format!("COTIZACIÓN  {}", s(quote, "folio")),
                    19.0,
                    true,
                    false,
                    INK,
                );
                paper.space(8.0);
                paper.pair("Cliente", s(quote, "customerName"), 10.0, false);
                paper.pair("Fecha", &date(s(quote, "createdAt")), 9.0, false);
                paper.pair("Válida hasta", &date(s(quote, "validUntil")), 10.0, true);
                if !quote["saleId"].is_null() {
                    paper.paragraph("Cotización convertida en venta.", 9.0, true, false, GRAY);
                }
                paper.space(14.0);
                let columns = [0.44, 0.16, 0.20, 0.20];
                paper.table_row(
                    &[
                        "Producto / clave".into(),
                        "Cantidad".into(),
                        "Precio antes de descuento".into(),
                        "Importe acordado".into(),
                    ],
                    &columns,
                    true,
                );
                for line in detail["lines"].as_array().into_iter().flatten() {
                    paper.table_row(
                        &[
                            format!("{}\n{}", s(line, "name"), s(line, "sku")),
                            format!(
                                "{} {}",
                                quantity(n(line, "quantityUnits")),
                                unit(n(line, "unit"))
                            ),
                            money(n(line, "priceCents")),
                            money(n(line, "totalCents")),
                        ],
                        &columns,
                        false,
                    );
                }
                paper.end_table();
                paper.space(14.0);
                paper.pair("IVA incluido", &money(n(quote, "taxCents")), 10.0, false);
                paper.pair("TOTAL ACORDADO", &money(n(quote, "totalCents")), 17.0, true);
                paper.space(7.0);
                paper.paragraph(
                    "Importes en MXN. Incluyen IVA y descuentos acordados.",
                    9.0,
                    false,
                    false,
                    GRAY,
                );
                if !s(quote, "notes").is_empty() {
                    paper.rule();
                    paper.paragraph("OBSERVACIONES", 10.0, true, false, INK);
                    paper.paragraph(s(quote, "notes"), 10.0, false, false, INK);
                }
                paper.space(12.0);
                paper.paragraph("Esta cotización no acredita un pago ni reserva existencias. La disponibilidad se comprueba al realizar la venta.",9.0,false,false,GRAY);
                paper.finish()
            }
            "purchase" => {
                let detail = self.purchase(request.id)?;
                let purchase = &detail["purchase"];
                let mut paper = Layout::new(s(purchase, "folio"), "purchase", 210.0, false, logo)?;
                branding(&mut paper, &settings)?;
                paper.paragraph(
                    &format!("RECEPCIÓN DE COMPRA  {}", s(purchase, "folio")),
                    17.0,
                    true,
                    false,
                    INK,
                );
                paper.space(10.0);
                paper.pair("Proveedor", s(purchase, "supplierName"), 10.0, true);
                paper.pair(
                    "Folio del proveedor",
                    s(purchase, "supplierReference"),
                    9.0,
                    false,
                );
                paper.pair("Fecha", &date(s(purchase, "createdAt")), 9.0, false);
                paper.space(12.0);
                let columns = [0.44, 0.20, 0.18, 0.18];
                paper.table_row(
                    &[
                        "Producto".into(),
                        "Unidades recibidas".into(),
                        "Costo por unidad".into(),
                        "Importe original".into(),
                    ],
                    &columns,
                    true,
                );
                for line in detail["lines"].as_array().into_iter().flatten() {
                    paper.table_row(
                        &[
                            format!("{}\n{}", s(line, "name"), s(line, "sku")),
                            format!(
                                "{} {}",
                                quantity(n(line, "quantityUnits")),
                                unit(n(line, "unit"))
                            ),
                            money(n(line, "costCents")),
                            if line["totalCents"].is_null() {
                                "Sin detalle original".into()
                            } else {
                                money(n(line, "totalCents"))
                            },
                        ],
                        &columns,
                        false,
                    );
                }
                paper.end_table();
                paper.space(14.0);
                paper.pair(
                    "TOTAL DE LA COMPRA",
                    &money(n(purchase, "totalCents")),
                    17.0,
                    true,
                );
                paper.space(8.0);
                paper.paragraph(if purchase["paidFromCash"]==true{"Pagada con efectivo de caja."}else if purchase["paidFromCash"].is_null(){"El registro histórico no contiene el desglose original por empaque ni la forma de pago."}else{"Sin retiro de efectivo de caja."},9.0,false,false,GRAY);
                paper.finish()
            }
            _ => Err(AppError::rule(
                "VALIDATION",
                "Selecciona un documento compatible.",
            )),
        }
    }
    pub fn export_document(&self, request: &DocumentRequest, folder: &Path) -> Result<Value> {
        ensure(
            folder.is_absolute() && folder.is_dir(),
            "INVALID_PATH",
            "Selecciona una carpeta existente para guardar el PDF.",
        )?;
        let document = self.business_document(request)?;
        let bytes = pdf::encode(&document)?;
        storage::require_space(folder, bytes.len() as u64)?;
        let stem: String = document
            .title
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .take(70)
            .collect();
        let nonce = uuid::Uuid::new_v4().to_string();
        let destination = folder.join(format!("{stem}-{}.pdf", &nonce[..8]));
        let partial = folder.join(format!(".DonovanPDF-{nonce}.partial"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            storage::move_new(&partial, &destination)?;
            Ok(
                json!({"path":storage::display_path(&destination),"bytes":bytes.len(),"pages":document.pages.len()}),
            )
        })();
        if partial.exists() {
            let _ = std::fs::remove_file(&partial);
        }
        result
    }
}
