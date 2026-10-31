use super::{
    date,
    layout::{BusinessDocument, GRAY, INK, Layout, PALE},
    s,
};
use crate::{
    error::{Result, ensure},
    reports::{Report, cell},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;

pub fn build(report: &Report, settings: &Value) -> Result<BusinessDocument> {
    let title = format!(
        "{} · {}",
        report.title,
        if report.current {
            "Saldo actual".into()
        } else {
            format!("{} — {}", date(&report.from), date(&report.to))
        }
    );
    let mut paper = Layout::landscape(&title, settings["logoData"].as_str().map(str::to_owned))?;
    let top = paper.y;
    if let Some(encoded) = paper.document.logo_data.as_deref() {
        let bytes = STANDARD.decode(encoded).map_err(|e| {
            crate::error::AppError::internal(
                "DOCUMENT_IMAGE",
                "No pudimos leer el logotipo del reporte.",
                e,
            )
        })?;
        crate::appearance::validate_image(&bytes)?;
        let logo = image::load_from_memory(&bytes).map_err(|e| {
            crate::error::AppError::internal(
                "DOCUMENT_IMAGE",
                "No pudimos leer el logotipo del reporte.",
                e,
            )
        })?;
        let aspect = logo.width() as f32 / logo.height() as f32;
        let width = 94.0f32.min(62.0 * aspect);
        let height = width / aspect;
        paper.logo_at(paper.width - paper.margin - width, top, width, height);
    }
    for (text, size, bold) in [
        (s(settings, "storeName"), 17.0, true),
        (s(settings, "subtitle"), 9.0, false),
    ] {
        for line in paper
            .fonts
            .wrap(text, paper.content_width() - 115.0, size, bold)
        {
            paper.text_at(paper.margin, paper.y + size, &line, size, bold, INK);
            paper.y += size * 1.4;
        }
    }
    if !s(settings, "taxId").is_empty() {
        paper.text_at(
            paper.margin,
            paper.y + 8.0,
            &format!("RFC: {}", s(settings, "taxId")),
            8.0,
            false,
            GRAY,
        );
        paper.y += 12.0;
    }
    paper.y = paper.y.max(top + 62.0) + 12.0;
    paper.paragraph(&report.title, 18.0, true, false, INK);
    paper.paragraph(
        &format!(
            "{} · Generado {}",
            if report.current {
                "Corte actual".into()
            } else {
                format!("{} — {}", date(&report.from), date(&report.to))
            },
            date(&report.generated_at)
        ),
        8.5,
        false,
        false,
        GRAY,
    );
    paper.space(5.0);
    paper.paragraph(&report.scope, 8.5, false, false, GRAY);
    if !report.search.is_empty() {
        paper.paragraph(
            &format!(
                "Búsqueda aplicada: {}. Los totales corresponden a las filas coincidentes.",
                report.search
            ),
            8.5,
            true,
            false,
            INK,
        );
    }
    paper.space(10.0);
    if !report.metrics.is_empty() {
        paper.ensure(58.0);
        let width = paper.content_width() / report.metrics.len() as f32;
        for (index, metric) in report.metrics.iter().enumerate() {
            let x = paper.margin + index as f32 * width;
            paper.rect(x, paper.y, width - 6.0, 54.0, PALE);
            for (line_index, line) in paper
                .fonts
                .wrap(&metric.label, width - 22.0, 8.0, false)
                .iter()
                .take(2)
                .enumerate()
            {
                paper.text_at(
                    x + 8.0,
                    paper.y + 13.0 + line_index as f32 * 10.0,
                    line,
                    8.0,
                    false,
                    GRAY,
                );
            }
            let value = metric
                .value
                .map(|v| {
                    if metric.kind == "money" {
                        super::layout::money(v)
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_else(|| "Pendiente".into());
            let mut size = 15.0;
            while paper.fonts.width(&value, size, true) > width - 22.0 && size > 9.0 {
                size -= 0.5;
            }
            paper.text_at(x + 8.0, paper.y + 43.0, &value, size, true, INK);
        }
        paper.space(66.0);
    }
    let total_weight: f32 = report.columns.iter().map(|c| c.weight).sum();
    let widths: Vec<f32> = report
        .columns
        .iter()
        .map(|c| c.weight / total_weight)
        .collect();
    paper.table_row(
        &report
            .columns
            .iter()
            .map(|c| c.label.clone())
            .collect::<Vec<_>>(),
        &widths,
        true,
    );
    for row in &report.rows {
        paper.table_row(
            &report
                .columns
                .iter()
                .map(|c| cell(&row[&c.key], c))
                .collect::<Vec<_>>(),
            &widths,
            false,
        );
        ensure(
            paper.document.pages.len() < 100,
            "DOCUMENT_LIMIT",
            "El reporte supera 100 páginas. Reduce el periodo o exporta el detalle completo a Excel.",
        )?;
    }
    paper.end_table();
    if report.rows.is_empty() {
        paper.paragraph(
            "No hay registros para esta selección.",
            10.0,
            false,
            false,
            GRAY,
        );
    }
    paper.space(10.0);
    paper.paragraph(
        &format!(
            "{} {} · MXN · Documento de control interno",
            report.rows.len(),
            if report.rows.len() == 1 {
                "fila"
            } else {
                "filas"
            }
        ),
        8.0,
        false,
        false,
        GRAY,
    );
    paper.finish()
}
