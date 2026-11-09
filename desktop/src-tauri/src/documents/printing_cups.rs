//! macOS CUPS adapter. PDF generation is shared with Windows and the preview.
use super::super::{
    layout::{BusinessDocument, MM},
    pdf,
};
use crate::error::{AppError, Result, ensure};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process::{Command, Output},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Printer {
    pub name: String,
    pub is_default: bool,
    pub virtual_printer: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintJob {
    pub job_id: i32,
    pub printer: String,
    pub pages: usize,
    pub message: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterDetails {
    pub name: String,
    pub driver: String,
    pub status: String,
    pub offline: bool,
    pub queued_jobs: u32,
    pub paper_widths_mm: Vec<f32>,
}
fn cups(program: &str, args: &[&str]) -> Result<Output> {
    Command::new(program).args(args).env("LC_ALL","C").output().map_err(|e|AppError::internal("PRINTER_SERVICE","No pudimos consultar el servicio de impresión de macOS. Puedes guardar el comprobante en PDF.",e))
}
fn paper_choices(options: &str) -> Vec<(f32, String)> {
    let mut choices = Vec::new();
    for line in options.lines().filter(|l| l.starts_with("PageSize/")) {
        if let Some((_, values)) = line.split_once(':') {
            for code in values.split_whitespace().map(|s| s.trim_start_matches('*')) {
                if !code
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                {
                    continue;
                }
                let lower = code.to_lowercase();
                for width in [58, 80] {
                    let n = width.to_string();
                    if lower == n
                        || lower.starts_with(&format!("{n}mm"))
                        || lower.starts_with(&format!("{n}x"))
                        || lower.starts_with(&format!("w{n}h"))
                    {
                        choices.push((width as f32, code.to_owned()));
                    }
                }
            }
        }
    }
    choices
}
pub fn printers() -> Result<Vec<Printer>> {
    let listing = cups("/usr/bin/lpstat", &["-p"])?;
    let text = String::from_utf8_lossy(&listing.stdout);
    if !listing.status.success() {
        if String::from_utf8_lossy(&listing.stderr).contains("No destinations added") {
            return Ok(vec![]);
        }
        return Err(AppError::rule(
            "PRINTER_SERVICE",
            "No se pudo consultar CUPS. Revisa Impresoras y escáneres en macOS.",
        ));
    }
    let default = cups("/usr/bin/lpstat", &["-d"])
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split(": ")
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_owned()
        })
        .unwrap_or_default();
    Ok(text
        .lines()
        .filter_map(|line| {
            line.strip_prefix("printer ")
                .and_then(|v| v.split_whitespace().next())
        })
        .map(|name| Printer {
            name: name.into(),
            is_default: name == default,
            virtual_printer: name.to_lowercase().contains("pdf"),
        })
        .collect())
}
pub fn details(selected: &str) -> Result<PrinterDetails> {
    let list = printers()?;
    let device = list
        .iter()
        .find(|p| {
            if selected.is_empty() {
                p.is_default
            } else {
                p.name == selected
            }
        })
        .ok_or_else(|| {
            AppError::rule(
                "PRINTER_MISSING",
                "Selecciona una impresora instalada en macOS. También puedes guardar el PDF.",
            )
        })?;
    let status = cups("/usr/bin/lpstat", &["-p", &device.name])?;
    let text = String::from_utf8_lossy(&status.stdout);
    let paused = text.contains("disabled");
    let options = cups("/usr/bin/lpoptions", &["-p", &device.name, "-l"])?;
    let options = String::from_utf8_lossy(&options.stdout);
    let mut widths: Vec<f32> = paper_choices(&options)
        .into_iter()
        .map(|(w, _)| w)
        .collect();
    widths.sort_by(f32::total_cmp);
    widths.dedup();
    let jobs = cups(
        "/usr/bin/lpstat",
        &["-W", "not-completed", "-o", &device.name],
    )
    .ok()
    .map(|o| String::from_utf8_lossy(&o.stdout).lines().count() as u32)
    .unwrap_or(0);
    Ok(PrinterDetails {
        name: device.name.clone(),
        driver: "CUPS / controlador de macOS".into(),
        status: if paused {
            "Cola pausada; revisa la impresora en macOS"
        } else {
            "Cola disponible; el controlador puede no informar la conexión física"
        }
        .into(),
        offline: paused,
        queued_jobs: jobs,
        paper_widths_mm: widths,
    })
}
pub fn print(source: &BusinessDocument, selected: &str, output: Option<&Path>) -> Result<PrintJob> {
    print_copies(source, selected, output, 1)
}
pub fn print_copies(
    source: &BusinessDocument,
    selected: &str,
    output: Option<&Path>,
    copies: i64,
) -> Result<PrintJob> {
    ensure(
        (1..=3).contains(&copies),
        "VALIDATION",
        "Selecciona entre una y tres copias.",
    )?;
    ensure(
        !source.pages.is_empty(),
        "DOCUMENT_EMPTY",
        "El documento no contiene páginas.",
    )?;
    if let Some(path) = output {
        ensure(
            path.is_absolute(),
            "INVALID_PATH",
            "Selecciona una ubicación absoluta.",
        )?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&pdf::encode(source)?)?;
        file.sync_all()?;
        return Ok(PrintJob {
            job_id: 0,
            printer: "PDF".into(),
            pages: source.pages.len(),
            message: "Documento PDF guardado".into(),
        });
    }
    let info = details(selected)?;
    ensure(
        !info.offline,
        "PRINTER_OFFLINE",
        "La cola está pausada. La venta permanece registrada; revisa la impresora o guarda el PDF.",
    )?;
    let folder = std::env::temp_dir().join(format!("DonovanPOS-print-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&folder)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700))?;
    }
    let result = (|| {
        let path = folder.join("comprobante.pdf");
        fs::write(&path, pdf::encode(source)?)?;
        let mut command = Command::new("/usr/bin/lp");
        command.env("LC_ALL", "C").args([
            "-d",
            &info.name,
            "-n",
            &copies.to_string(),
            "-t",
            "Donovan POS Pro",
        ]);
        let page = &source.pages[0];
        let width = page.width / MM;
        let options = cups("/usr/bin/lpoptions", &["-p", &info.name, "-l"])?;
        if let Some((_, code)) = paper_choices(&String::from_utf8_lossy(&options.stdout))
            .into_iter()
            .find(|(w, _)| (width - *w).abs() < 1.)
        {
            command.args(["-o", &format!("PageSize={code}")]);
        }
        let submitted = command.arg("--").arg(&path).output()?;
        ensure(
            submitted.status.success(),
            "PRINT_FAILED",
            "CUPS no aceptó la impresión. La venta ya está registrada; reimprime el mismo folio o guarda el PDF.",
        )?;
        let text = String::from_utf8_lossy(&submitted.stdout);
        let job_id = text
            .split_whitespace()
            .find_map(|part| {
                part.strip_prefix(&format!("{}-", info.name))
                    .and_then(|n| n.parse::<i32>().ok())
            })
            .unwrap_or(0);
        Ok(PrintJob {
            job_id,
            printer: info.name,
            pages: source.pages.len(),
            message: "Ticket enviado a la cola de macOS. Comprueba la salida en la impresora."
                .into(),
        })
    })();
    let _ = fs::remove_file(folder.join("comprobante.pdf"));
    let _ = fs::remove_dir(folder);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_selects_sizes_advertised_by_the_driver() {
        assert_eq!(
            paper_choices("PageSize/Media Size: *A4 A5 58mm 80x297mm"),
            vec![(58., "58mm".into()), (80., "80x297mm".into())]
        );
        assert!(
            paper_choices("PageSize/Media Size: A4 Custom.WIDTHxHEIGHT\nResolution: 580dpi 800dpi")
                .is_empty()
        );
    }
}
