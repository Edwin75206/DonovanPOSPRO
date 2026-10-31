use super::{
    layout::{BusinessDocument, Draw, Fonts, MM, Page},
    s,
};
use crate::{
    error::{AppError, Result, ensure},
    models::Product,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;

fn text(page: &mut Page, fonts: &Fonts, x: f32, y: f32, value: &str, size: f32, bold: bool) {
    let text: String = value
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| {
            if fonts.face(bold).glyph_index(c).is_some() {
                c
            } else {
                '?'
            }
        })
        .collect();
    let width = fonts.width(&text, size, bold);
    page.operations.push(Draw::Text {
        x,
        y,
        text,
        size,
        bold,
        width,
        color: [0, 0, 0],
    });
}
fn rect(page: &mut Page, x: f32, y: f32, width: f32, height: f32, color: [u8; 3]) {
    page.operations.push(Draw::Rect {
        x,
        y,
        width,
        height,
        color,
    });
}
fn short_lines(
    fonts: &Fonts,
    value: &str,
    width: f32,
    size: f32,
    bold: bool,
    max: usize,
) -> Vec<String> {
    let mut lines = fonts.wrap(value, width, size, bold);
    if lines.len() > max {
        lines.truncate(max);
        let last = lines.last_mut().unwrap();
        while !last.is_empty() && fonts.width(&format!("{last}…"), size, bold) > width {
            last.pop();
        }
        last.push('…');
    }
    lines
}
pub fn build(product: &Product, settings: &Value, copies: i64) -> Result<BusinessDocument> {
    ensure(
        (1..=500).contains(&copies),
        "VALIDATION",
        "Selecciona entre 1 y 500 etiquetas.",
    )?;
    ensure(
        product.active,
        "VALIDATION",
        "Activa el producto antes de imprimir etiquetas para venta.",
    )?;
    let code = product
        .barcode
        .as_deref()
        .filter(|c| !c.is_empty())
        .unwrap_or(&product.sku);
    ensure(
        !code.is_empty() && code.bytes().all(|b| (32..=126).contains(&b)),
        "LABEL_CODE",
        "Define un código de barras con letras sin acentos, números o signos ASCII para generar una etiqueta escaneable.",
    )?;
    // Numeric pairs reduce the width without changing the data returned by a scanner.
    let encoded = if code.bytes().all(|b| b.is_ascii_digit()) && code.len() >= 4 {
        let even = code.len() / 2 * 2;
        format!(
            "Ć{}{}",
            &code[..even],
            if even < code.len() {
                format!("Ɓ{}", &code[even..])
            } else {
                String::new()
            }
        )
    } else {
        format!("Ɓ{code}")
    };
    let bars = barcoders::sym::code128::Code128::new(encoded)
        .map_err(|_| {
            AppError::rule(
                "LABEL_CODE",
                "El código del producto no se puede representar en esta etiqueta.",
            )
        })?
        .encode();
    let module = (81.0 / (bars.len() + 20) as f32).min(0.38) * MM;
    ensure(
        module >= 0.25 * MM,
        "LABEL_CODE",
        "El código es demasiado largo para una etiqueta legible de este tamaño. Asigna un código de barras más corto al producto.",
    )?;
    let fonts = Fonts::load()?;
    let logo = settings["logoData"].as_str().map(str::to_owned);
    let aspect = if let Some(encoded) = logo.as_deref() {
        let bytes = STANDARD.decode(encoded).map_err(|e| {
            AppError::internal(
                "DOCUMENT_IMAGE",
                "No pudimos leer el logotipo de las etiquetas.",
                e,
            )
        })?;
        crate::appearance::validate_image(&bytes)?;
        let image = image::load_from_memory(&bytes).map_err(|e| {
            AppError::internal(
                "DOCUMENT_IMAGE",
                "No pudimos leer el logotipo de las etiquetas.",
                e,
            )
        })?;
        Some(image.width() as f32 / image.height() as f32)
    } else {
        None
    };
    let pages = (copies as usize).div_ceil(16);
    let mut document=BusinessDocument{title:format!("Etiquetas-{}",product.sku),kind:"labels".into(),pages:vec![],logo_data:logo,warnings:vec!["Hoja A4: 16 etiquetas de 91 × 33 mm. Imprime al 100%, sin ajustar, y recorta por las guías. Las descripciones largas se abrevian; el código y el precio se conservan.".into()]};
    for index in 0..pages {
        let mut page = Page {
            width: 210.0 * MM,
            height: 297.0 * MM,
            margin: 12.0 * MM,
            operations: vec![],
        };
        for slot in 0..16usize {
            if index * 16 + slot >= copies as usize {
                break;
            }
            let x = (12.0 + (slot % 2) as f32 * 95.0) * MM;
            let y = (12.0 + (slot / 2) as f32 * 34.0) * MM;
            let w = 91.0 * MM;
            let h = 33.0 * MM;
            for (rx, ry, rw, rh) in [
                (x, y, w, 0.25),
                (x, y + h, w, 0.25),
                (x, y, 0.25, h),
                (x + w, y, 0.25, h),
            ] {
                rect(&mut page, rx, ry, rw, rh, [215, 215, 215]);
            }
            let business_x = if let Some(aspect) = aspect {
                let lw = (12.0 * MM).min(7.5 * MM * aspect);
                let lh = lw / aspect;
                page.operations.push(Draw::Logo {
                    x: x + 3.0 * MM,
                    y: y + 1.5 * MM,
                    width: lw,
                    height: lh,
                });
                x + 18.0 * MM
            } else {
                x + 3.0 * MM
            };
            for (i, line) in short_lines(
                &fonts,
                s(settings, "storeName"),
                x + w - 3.0 * MM - business_x,
                6.5,
                true,
                2,
            )
            .iter()
            .enumerate()
            {
                text(
                    &mut page,
                    &fonts,
                    business_x,
                    y + (5.0 + i as f32 * 2.6) * MM,
                    line,
                    6.5,
                    true,
                );
            }
            for (i, line) in short_lines(&fonts, &product.name, w - 6.0 * MM, 8.0, false, 2)
                .iter()
                .enumerate()
            {
                text(
                    &mut page,
                    &fonts,
                    x + 3.0 * MM,
                    y + (12.0 + i as f32 * 3.3) * MM,
                    line,
                    8.0,
                    false,
                );
            }
            let price = super::layout::money(product.price_cents);
            text(
                &mut page,
                &fonts,
                x + 3.0 * MM,
                y + 21.0 * MM,
                &price,
                14.0,
                true,
            );
            let unit = format!("{} · IVA incluido", super::layout::unit(product.unit));
            let unit_width = fonts.width(&unit, 6.5, false);
            text(
                &mut page,
                &fonts,
                x + w - 3.0 * MM - unit_width,
                y + 21.0 * MM,
                &unit,
                6.5,
                false,
            );
            let start = x + (w - bars.len() as f32 * module) / 2.0;
            let mut bit = 0;
            while bit < bars.len() {
                if bars[bit] == 0 {
                    bit += 1;
                    continue;
                }
                let first = bit;
                while bit < bars.len() && bars[bit] != 0 {
                    bit += 1;
                }
                rect(
                    &mut page,
                    start + first as f32 * module,
                    y + 22.5 * MM,
                    (bit - first) as f32 * module,
                    7.0 * MM,
                    [0, 0, 0],
                );
            }
            let code_width = fonts.width(code, 6.5, false);
            text(
                &mut page,
                &fonts,
                x + (w - code_width) / 2.0,
                y + 32.0 * MM,
                code,
                6.5,
                false,
            );
        }
        let footer = format!(
            "Donovan POS Pro · Etiquetas · {} / {} · A4 / 100%",
            index + 1,
            pages
        );
        let width = fonts.width(&footer, 6.5, false);
        text(
            &mut page,
            &fonts,
            (210.0 * MM - width) / 2.0,
            286.0 * MM,
            &footer,
            6.5,
            false,
        );
        document.pages.push(page);
    }
    Ok(document)
}
