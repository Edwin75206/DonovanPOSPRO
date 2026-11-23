//! Windows spooler jobs render the same layout as the preview/PDF. No database
//! connection or checkout operation is involved while a printer driver is running.
use super::super::layout::{BOLD, BusinessDocument, Draw, Fonts, MM, Page, REGULAR};
use crate::error::{AppError, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;
use std::{
    mem::size_of,
    path::Path,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{HANDLE, POINT, RECT},
    Graphics::{Gdi::*, Printing::*},
    Storage::Xps::*,
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
    #[serde(skip)]
    rpt004_escpos: bool,
}
pub fn status_message(status: u32, attributes: u32) -> (String, bool) {
    let offline =
        status & PRINTER_STATUS_OFFLINE != 0 || attributes & PRINTER_ATTRIBUTE_WORK_OFFLINE != 0;
    let mut messages = Vec::new();
    if offline {
        messages.push("Sin conexión");
    }
    for (flag, message) in [
        (PRINTER_STATUS_PAPER_OUT, "Sin papel"),
        (PRINTER_STATUS_PAPER_JAM, "Papel atascado"),
        (PRINTER_STATUS_PAUSED, "En pausa"),
        (PRINTER_STATUS_ERROR, "Error del dispositivo"),
        (PRINTER_STATUS_DOOR_OPEN, "Tapa abierta"),
        (PRINTER_STATUS_PRINTING, "Imprimiendo"),
    ] {
        if status & flag != 0 {
            messages.push(message);
        }
    }
    if messages.is_empty() {
        messages.push("Sin incidencias informadas por Windows");
    }
    (messages.join(" · "), offline)
}
pub fn details(selected: &str) -> Result<PrinterDetails> {
    let devices = printers()?;
    let device = devices
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
                "Esta impresora no está disponible. Selecciona otra o guarda el ticket como PDF.",
            )
        })?;
    unsafe {
        let name = wide(&device.name);
        let mut raw = PRINTER_HANDLE::default();
        if OpenPrinterW(name.as_ptr(), &mut raw, null()) == 0 {
            return Err(error("OpenPrinterW"));
        }
        let handle = PrinterHandle(raw);
        let mut needed = 0;
        GetPrinterW(handle.0, 2, null_mut(), 0, &mut needed);
        ensure(
            needed >= size_of::<PRINTER_INFO_2W>() as u32 && needed < 8 * 1024 * 1024,
            "PRINT_FAILED",
            "No pudimos consultar el controlador de impresión.",
        )?;
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
        if GetPrinterW(handle.0, 2, buffer.as_mut_ptr().cast(), needed, &mut needed) == 0 {
            return Err(error("GetPrinterW"));
        }
        let info = &*buffer.as_ptr().cast::<PRINTER_INFO_2W>();
        let (status, offline) = status_message(info.Status, info.Attributes);
        let count = DeviceCapabilitiesW(
            name.as_ptr(),
            info.pPortName,
            DC_PAPERSIZE,
            null_mut(),
            null(),
        );
        let mut widths = Vec::new();
        if count > 0 && count < 4096 {
            let mut sizes = vec![POINT { x: 0, y: 0 }; count as usize];
            let read = DeviceCapabilitiesW(
                name.as_ptr(),
                info.pPortName,
                DC_PAPERSIZE,
                sizes.as_mut_ptr().cast(),
                null(),
            );
            if read > 0 && read <= count {
                for size in sizes.iter().take(read as usize) {
                    let width = size.x as f32 / 10.;
                    if width > 0. && !widths.contains(&width) {
                        widths.push(width);
                    }
                }
            }
        }
        widths.sort_by(f32::total_cmp);
        let driver = read_wide(info.pDriverName);
        let rpt004_escpos = driver == "Generic / Text Only"
            && read_wide(info.pComment) == "3nStar RPT004 ESC/POS";
        if rpt004_escpos {
            widths = vec![80.0];
        }
        Ok(PrinterDetails {
            name: device.name.clone(),
            driver,
            status,
            offline,
            queued_jobs: info.cJobs,
            paper_widths_mm: widths,
            rpt004_escpos,
        })
    }
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn error(action: &str) -> AppError {
    AppError::internal(
        "PRINT_FAILED",
        "No pudimos confirmar la impresión. Revisa la cola de Windows antes de volver a enviar el documento; la operación guardada se conserva.",
        format!("{action}: {}", std::io::Error::last_os_error()),
    )
}
unsafe fn read_wide(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut len = 0;
    while len < 32768 && unsafe { *pointer.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, len) })
}
pub fn printers() -> Result<Vec<Printer>> {
    unsafe {
        let mut default_len = 0;
        GetDefaultPrinterW(null_mut(), &mut default_len);
        let mut default = vec![0u16; default_len.min(32768) as usize];
        let default_name = if !default.is_empty()
            && GetDefaultPrinterW(default.as_mut_ptr(), &mut default_len) != 0
        {
            read_wide(default.as_ptr())
        } else {
            String::new()
        };
        let mut needed = 0;
        let mut count = 0;
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let listed = EnumPrintersW(flags, null(), 4, null_mut(), 0, &mut needed, &mut count);
        if needed == 0 {
            return if listed != 0 {
                Ok(vec![])
            } else {
                Err(error("EnumPrintersW"))
            };
        }
        ensure(
            needed < 8 * 1024 * 1024,
            "PRINT_FAILED",
            "Windows devolvió una lista de impresoras demasiado grande.",
        )?;
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
        if EnumPrintersW(
            flags,
            null(),
            4,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
            &mut count,
        ) == 0
        {
            return Err(error("EnumPrintersW"));
        }
        ensure(
            count as usize <= buffer.len() * 8 / size_of::<PRINTER_INFO_4W>(),
            "PRINT_FAILED",
            "No pudimos leer la lista de impresoras de Windows.",
        )?;
        let entries =
            std::slice::from_raw_parts(buffer.as_ptr().cast::<PRINTER_INFO_4W>(), count as usize);
        let mut result: Vec<Printer> = entries
            .iter()
            .map(|entry| {
                let name = read_wide(entry.pPrinterName);
                let lower = name.to_lowercase();
                Printer {
                    is_default: name == default_name,
                    virtual_printer: ["pdf", "xps", "onenote", "fax"]
                        .iter()
                        .any(|s| lower.contains(s)),
                    name,
                }
            })
            .collect();
        result.sort_by(|a, b| b.is_default.cmp(&a.is_default).then(a.name.cmp(&b.name)));
        Ok(result)
    }
}
struct PrinterHandle(PRINTER_HANDLE);
impl Drop for PrinterHandle {
    fn drop(&mut self) {
        unsafe {
            ClosePrinter(self.0);
        }
    }
}
struct Dc(HDC);
impl Drop for Dc {
    fn drop(&mut self) {
        unsafe {
            DeleteDC(self.0);
        }
    }
}
struct FontResource(HANDLE);
impl Drop for FontResource {
    fn drop(&mut self) {
        unsafe {
            RemoveFontMemResourceEx(self.0);
        }
    }
}
struct Job {
    dc: HDC,
    completed: bool,
}
impl Drop for Job {
    fn drop(&mut self) {
        if !self.completed {
            unsafe {
                AbortDoc(self.dc);
            }
        }
    }
}
fn font_resource(bytes: &[u8]) -> Result<FontResource> {
    let mut count = 0;
    let handle = unsafe {
        AddFontMemResourceEx(
            bytes.as_ptr().cast(),
            bytes.len() as u32,
            null(),
            &mut count,
        )
    };
    if handle.is_null() {
        Err(error("AddFontMemResourceEx"))
    } else {
        Ok(FontResource(handle))
    }
}
fn color(rgb: [u8; 3]) -> u32 {
    rgb[0] as u32 | ((rgb[1] as u32) << 8) | ((rgb[2] as u32) << 16)
}

type Logo = (Vec<u8>, u32, u32);

fn decode_logo(source: &BusinessDocument) -> Result<Option<Logo>> {
    source
        .logo_data
        .as_deref()
        .map(|encoded| -> Result<(Vec<u8>, u32, u32)> {
            let bytes = STANDARD.decode(encoded).map_err(|e| {
                AppError::internal(
                    "DOCUMENT_IMAGE",
                    "No pudimos preparar el logotipo para imprimir.",
                    e,
                )
            })?;
            crate::appearance::validate_image(&bytes)?;
            let decoded = image::load_from_memory(&bytes)
                .map_err(|e| {
                    AppError::internal("DOCUMENT_IMAGE", "No pudimos leer el logotipo.", e)
                })?
                .thumbnail(1600, 1600)
                .to_rgba8();
            let mut pixels = Vec::with_capacity(decoded.len());
            for p in decoded.pixels() {
                let a = p[3] as u32;
                for channel in [p[2], p[1], p[0]] {
                    pixels.push(((channel as u32 * a + 255 * (255 - a) + 127) / 255) as u8);
                }
                pixels.push(255);
            }
            Ok((pixels, decoded.width(), decoded.height()))
        })
        .transpose()
}

struct PageTransform {
    left: f32,
    top: f32,
    scale: f32,
    dpi_x: f32,
    dpi_y: f32,
    thermal_text: bool,
}

fn render_page(
    dc: HDC,
    page: &Page,
    fonts: &Fonts,
    logo: Option<&Logo>,
    transform: PageTransform,
) -> Result<()> {
    let PageTransform {
        left,
        top,
        scale,
        dpi_x,
        dpi_y,
        thermal_text,
    } = transform;
    let family = wide("Donovan Paper");
    let px = |x: f32| ((left + (x - page.margin) * scale) * dpi_x / 72.0).round() as i32;
    let py = |y: f32| ((top + (y - page.margin) * scale) * dpi_y / 72.0).round() as i32;
    unsafe {
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextAlign(dc, TA_BASELINE);
        for op in &page.operations {
            match op {
                Draw::Text {
                    x,
                    y,
                    text,
                    size,
                    bold,
                    color: ink,
                    ..
                } => {
                    let font = CreateFontW(
                        -(*size * scale * dpi_y / 72.0).round() as i32,
                        0,
                        0,
                        0,
                        if *bold { 700 } else { 400 },
                        0,
                        0,
                        0,
                        DEFAULT_CHARSET as u32,
                        OUT_TT_ONLY_PRECIS as u32,
                        CLIP_DEFAULT_PRECIS as u32,
                        ANTIALIASED_QUALITY as u32,
                        0,
                        family.as_ptr(),
                    );
                    if font.is_null() {
                        return Err(error("CreateFontW"));
                    }
                    let previous = SelectObject(dc, font);
                    let mut selected_family = [0u16; 128];
                    if GetTextFaceW(
                        dc,
                        selected_family.len() as i32,
                        selected_family.as_mut_ptr(),
                    ) <= 0
                        || read_wide(selected_family.as_ptr()) != "Donovan Paper"
                    {
                        SelectObject(dc, previous);
                        DeleteObject(font);
                        return Err(AppError::rule(
                            "PRINT_FONT",
                            "La impresora no pudo cargar la tipografía del documento. Guarda el PDF para conservar su presentación.",
                        ));
                    }
                    SetTextColor(dc, color(if thermal_text { [0, 0, 0] } else { *ink }));
                    let glyphs: Vec<u16> =
                        text.chars().map(|c| fonts.glyph(c, *bold).0).collect();
                    let mut cumulative = 0.0f32;
                    let mut rounded = 0i32;
                    let dx: Vec<i32> = text
                        .chars()
                        .map(|c| {
                            cumulative +=
                                fonts.width(&c.to_string(), *size, *bold) * scale * dpi_x
                                    / 72.0;
                            let next = cumulative.round() as i32;
                            let step = next - rounded;
                            rounded = next;
                            step
                        })
                        .collect();
                    let success = ExtTextOutW(
                        dc,
                        px(*x),
                        py(*y),
                        ETO_GLYPH_INDEX,
                        null(),
                        glyphs.as_ptr(),
                        glyphs.len() as u32,
                        dx.as_ptr(),
                    );
                    SelectObject(dc, previous);
                    DeleteObject(font);
                    if success == 0 {
                        return Err(error("ExtTextOutW"));
                    }
                }
                Draw::Rect {
                    x,
                    y,
                    width,
                    height,
                    color: ink,
                } => {
                    let brush = CreateSolidBrush(color(*ink));
                    if brush.is_null() {
                        return Err(error("CreateSolidBrush"));
                    }
                    let rect = RECT {
                        left: px(*x),
                        top: py(*y),
                        right: px(x + width).max(px(*x) + 1),
                        bottom: py(y + height).max(py(*y) + 1),
                    };
                    let success = FillRect(dc, &rect, brush);
                    DeleteObject(brush);
                    if success == 0 {
                        return Err(error("FillRect"));
                    }
                }
                Draw::Logo {
                    x,
                    y,
                    width,
                    height,
                } => {
                    if let Some((pixels, w, h)) = logo {
                        let bitmap = BITMAPINFO {
                            bmiHeader: BITMAPINFOHEADER {
                                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                                biWidth: *w as i32,
                                biHeight: -(*h as i32),
                                biPlanes: 1,
                                biBitCount: 32,
                                biCompression: BI_RGB,
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        let result = StretchDIBits(
                            dc,
                            px(*x),
                            py(*y),
                            px(x + width) - px(*x),
                            py(y + height) - py(*y),
                            0,
                            0,
                            *w as i32,
                            *h as i32,
                            pixels.as_ptr().cast(),
                            &bitmap,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        );
                        if result == 0 || result == -1 {
                            return Err(error("StretchDIBits"));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

// The RPT004 has a 576-dot head on 80 mm paper. The marked Windows queue
// transports ESC/POS bytes over USB using Microsoft's built-in driver.
const RPT004_WIDTH_DOTS: usize = 576;
const RPT004_DOTS_PER_MM: f32 = 8.0;

struct Dib {
    dc: Dc,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
}

impl Dib {
    fn new(width: usize, height: usize) -> Result<Self> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            let dc = Dc(CreateCompatibleDC(null_mut()));
            if dc.0.is_null() {
                return Err(error("CreateCompatibleDC"));
            }
            let mut bits = null_mut();
            let bitmap = CreateDIBSection(dc.0, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bitmap.is_null() {
                return Err(error("CreateDIBSection"));
            }
            let previous = SelectObject(dc.0, bitmap);
            if previous.is_null() {
                let failure = error("SelectObject");
                DeleteObject(bitmap);
                return Err(failure);
            }
            let surface = Self {
                dc,
                bitmap,
                previous,
                bits: bits.cast(),
            };
            if PatBlt(surface.dc.0, 0, 0, width as i32, height as i32, WHITENESS) == 0 {
                return Err(error("PatBlt"));
            }
            Ok(surface)
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc.0, self.previous);
            DeleteObject(self.bitmap);
        }
    }
}

fn append_rpt004_raster(
    data: &mut Vec<u8>,
    page: &Page,
    fonts: &Fonts,
    logo: Option<&Logo>,
) -> Result<()> {
    let height = (page.height / MM * RPT004_DOTS_PER_MM).ceil() as usize;
    ensure(
        (page.width / MM - 80.0).abs() < 0.1 && (1..=2400).contains(&height),
        "PRINTER_PAPER",
        "Selecciona papel de 80 mm para el ticket de la RPT004.",
    )?;
    let surface = Dib::new(RPT004_WIDTH_DOTS, height)?;
    let usable_width = RPT004_WIDTH_DOTS as f32 / RPT004_DOTS_PER_MM * MM;
    let content_width = page.width - 2.0 * page.margin;
    let scale = (usable_width / content_width).min(1.0);
    let dpi = RPT004_DOTS_PER_MM * 25.4;
    render_page(
        surface.dc.0,
        page,
        fonts,
        logo,
        PageTransform {
            left: (usable_width - content_width * scale) / 2.0,
            top: page.margin,
            scale,
            dpi_x: dpi,
            dpi_y: dpi,
            thermal_text: true,
        },
    )?;
    let pixels = unsafe {
        if GdiFlush() == 0 {
            return Err(error("GdiFlush"));
        }
        std::slice::from_raw_parts(surface.bits, RPT004_WIDTH_DOTS * height * 4)
    };
    // Each strip stays well below the printer's receive buffer.
    let stride = RPT004_WIDTH_DOTS * 4;
    let byte_width = (RPT004_WIDTH_DOTS as u16 / 8).to_le_bytes();
    for strip in pixels.chunks(stride * 128) {
        let rows = ((strip.len() / stride) as u16).to_le_bytes();
        data.extend_from_slice(&[
            0x1d,
            b'v',
            b'0',
            0,
            byte_width[0],
            byte_width[1],
            rows[0],
            rows[1],
        ]);
        for group in strip.chunks_exact(8 * 4) {
            let mut dots = 0;
            for (index, pixel) in group.chunks_exact(4).enumerate() {
                let luminance =
                    2126 * pixel[2] as u32 + 7152 * pixel[1] as u32 + 722 * pixel[0] as u32;
                // Retain the antialiased edges of small letters on the thermal head.
                if luminance < 205 * 10000 {
                    dots |= 0x80 >> index;
                }
            }
            data.push(dots);
        }
    }
    Ok(())
}

struct RawJob {
    handle: PrinterHandle,
    complete: bool,
}

impl Drop for RawJob {
    fn drop(&mut self) {
        if !self.complete {
            unsafe { AbortPrinter(self.handle.0) };
        }
    }
}

fn print_rpt004(
    source: &BusinessDocument,
    printer: &str,
    copies: i64,
    fonts: &Fonts,
    logo: Option<&Logo>,
) -> Result<PrintJob> {
    let receipt = rpt004_receipt(source, fonts, logo)?;
    let size = u32::try_from(receipt.len())
        .map_err(|_| AppError::rule("PRINT_FAILED", "El ticket es demasiado grande para imprimir."))?;
    let name = wide(printer);
    let title = wide(&format!("Donovan POS Pro · {}", source.title));
    let raw = wide("RAW");
    unsafe {
        let mut handle = PRINTER_HANDLE::default();
        if OpenPrinterW(name.as_ptr(), &mut handle, null()) == 0 {
            return Err(error("OpenPrinterW"));
        }
        let handle = PrinterHandle(handle);
        let info = DOC_INFO_1W {
            pDocName: title.as_ptr().cast_mut(),
            pOutputFile: null_mut(),
            pDatatype: raw.as_ptr().cast_mut(),
        };
        let id = StartDocPrinterW(handle.0, 1, &info);
        if id == 0 {
            return Err(error("StartDocPrinterW"));
        }
        let mut job = RawJob {
            handle,
            complete: false,
        };
        for _ in 0..copies {
            let mut written = 0;
            if WritePrinter(job.handle.0, receipt.as_ptr().cast(), size, &mut written) == 0 {
                return Err(error("WritePrinter"));
            }
            ensure(
                written == size,
                "PRINT_FAILED",
                "Windows recibió solo parte del ticket. Revisa la cola de impresión antes de reenviarlo.",
            )?;
        }
        if EndDocPrinter(job.handle.0) == 0 {
            return Err(error("EndDocPrinter"));
        }
        job.complete = true;
        Ok(PrintJob {
            job_id: id as i32,
            printer: printer.into(),
            pages: source.pages.len() * copies as usize,
            message: "Windows recibió el ticket con la instrucción de corte automático.".into(),
        })
    }
}

fn rpt004_receipt(source: &BusinessDocument, fonts: &Fonts, logo: Option<&Logo>) -> Result<Vec<u8>> {
    ensure(
        source.kind == "sale",
        "PRINTER_PAPER",
        "La RPT004 imprime tickets. Elige una impresora de hojas o guarda este documento en PDF.",
    )?;
    let mut receipt = vec![0x1b, b'@'];
    for page in &source.pages {
        append_rpt004_raster(&mut receipt, page, fonts, logo)?;
    }
    // GS V 66 0 feeds to the blade and makes a partial cut.
    receipt.extend_from_slice(&[0x1d, b'V', 66, 0]);
    Ok(receipt)
}

#[cfg(test)]
mod rpt004_tests {
    use super::*;
    use crate::documents::layout::{INK, Layout};

    #[test]
    fn renders_80_mm_receipt_and_places_cut_after_last_raster_strip() {
        let _regular = font_resource(REGULAR).unwrap();
        let _bold = font_resource(BOLD).unwrap();
        let mut layout = Layout::new("Control", "sale", 80.0, true, None).unwrap();
        layout.paragraph("Comprobante · Construrama", 11.0, true, true, INK);
        let document = layout.finish().unwrap();
        let receipt = rpt004_receipt(&document, &Fonts::load().unwrap(), None).unwrap();
        let commands = receipt.strip_prefix(&[0x1b, b'@']).unwrap();
        let mut strips = commands.strip_suffix(&[0x1d, b'V', 66, 0]).unwrap();
        let mut nonwhite = false;
        let mut rows = 0;
        while !strips.is_empty() {
            assert_eq!(&strips[..6], &[0x1d, b'v', b'0', 0, 72, 0]);
            let count = u16::from_le_bytes([strips[6], strips[7]]) as usize;
            assert!((1..=128).contains(&count));
            let (pixels, remaining) = strips[8..].split_at(72 * count);
            nonwhite |= pixels.iter().any(|&byte| byte != 0);
            rows += count;
            strips = remaining;
        }
        assert!(nonwhite);
        assert_eq!(rows, 400);
    }

    #[test]
    #[ignore = "Prints on a configured 3nStar RPT004 USB queue"]
    fn configured_rpt004_prints_a_formatted_ticket() {
        let name = "3nStar RPT004";
        let status = details(name).unwrap();
        assert!(status.rpt004_escpos && !status.offline);
        let logo = STANDARD.encode(include_bytes!(
            "../../../../assets/branding/customer-logo-transparent.png"
        ));
        let logo = crate::documents::thermal_logo(Some(logo), true).unwrap();
        let mut layout = Layout::new("Verificación de corte", "sale", 80.0, true, logo).unwrap();
        layout.logo(1.5);
        layout.paragraph("Construrama Chicontla", 11.0, true, true, INK);
        layout.rule();
        layout.paragraph("Verificación de corte automático", 8.0, false, true, INK);
        let job = print(&layout.finish().unwrap(), name, None).unwrap();
        assert!(job.job_id > 0);
    }
}

/// `output` is only used by the isolated native integration check. The desktop
/// command never accepts filesystem paths supplied by JavaScript for printing.
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
    // Some drivers cannot report status. GDI still decides whether a job can be
    // accepted; only a known offline state prevents submission here.
    let status = details(selected).ok();
    if let Some(status) = &status {
        ensure(
            !status.offline,
            "PRINTER_OFFLINE",
            "La impresora está sin conexión. La venta permanece registrada; reconecta el equipo o guarda el PDF.",
        )?;
    }
    ensure(
        !source.pages.is_empty(),
        "DOCUMENT_EMPTY",
        "El documento no contiene páginas para imprimir.",
    )?;
    let available = printers()?;
    let printer=available.iter().find(|p|if selected.trim().is_empty(){p.is_default}else{p.name==selected}).ok_or_else(||AppError::rule("PRINTER_MISSING","Selecciona una impresora instalada en Windows. También puedes guardar el documento como PDF."))?;
    if let Some(output) = output {
        ensure(
            output.is_absolute() && !output.exists(),
            "FILE_EXISTS",
            "Selecciona un archivo nuevo. No se sobrescribirá un documento existente.",
        )?;
    }
    // Virtual printers normally display an additional save dialog. The app uses
    // its explicit PDF export instead; this path cannot silently open that UI.
    ensure(
        !printer.virtual_printer || output.is_some(),
        "PRINTER_VIRTUAL",
        "Esta es una impresora virtual. Usa Guardar PDF para elegir la carpeta y conservar el tamaño del documento.",
    )?;
    let _regular = font_resource(REGULAR)?;
    let _bold = font_resource(BOLD)?;
    let fonts = Fonts::load()?;
    let logo = decode_logo(source)?;
    if output.is_none() && status.is_some_and(|details| details.rpt004_escpos) {
        return print_rpt004(source, &printer.name, copies, &fonts, logo.as_ref());
    }
    let name = wide(&printer.name);
    let driver = wide("WINSPOOL");
    let title = wide(&format!("Donovan POS Pro · {}", source.title));
    let output = output.map(|p| wide(&p.to_string_lossy()));
    unsafe {
        let mut handle = PRINTER_HANDLE::default();
        if OpenPrinterW(name.as_ptr(), &mut handle, null()) == 0 {
            return Err(error("OpenPrinterW"));
        }
        let handle = PrinterHandle(handle);
        let bytes = DocumentPropertiesW(null_mut(), handle.0, name.as_ptr(), null_mut(), null(), 0);
        ensure(
            bytes >= size_of::<DEVMODEW>() as i32 && bytes < 1024 * 1024,
            "PRINT_FAILED",
            "No pudimos leer la configuración de la impresora.",
        )?;
        let mut mode = vec![0u64; (bytes as usize).div_ceil(8)];
        let mode_ptr = mode.as_mut_ptr().cast::<DEVMODEW>();
        if DocumentPropertiesW(
            null_mut(),
            handle.0,
            name.as_ptr(),
            mode_ptr,
            null(),
            DM_OUT_BUFFER,
        ) != 1
        {
            return Err(error("DocumentPropertiesW"));
        }
        let apply_size = |width: f32, height: f32| -> Result<()> {
            let mode = &mut *mode_ptr;
            let landscape = source.kind == "report" && width > height;
            let (paper_width, paper_height) = if landscape {
                (height, width)
            } else {
                (width, height)
            };
            mode.dmFields |=
                DM_ORIENTATION | DM_PAPERSIZE | DM_PAPERWIDTH | DM_PAPERLENGTH | DM_COPIES;
            mode.Anonymous1.Anonymous1.dmOrientation = if landscape {
                DMORIENT_LANDSCAPE
            } else {
                DMORIENT_PORTRAIT
            } as i16;
            mode.Anonymous1.Anonymous1.dmCopies = 1;
            mode.Anonymous1.Anonymous1.dmPaperSize = if (paper_width / MM - 210.0).abs() < 0.1
                && (paper_height / MM - 297.0).abs() < 0.1
            {
                DMPAPER_A4 as i16
            } else {
                DMPAPER_USER as i16
            };
            mode.Anonymous1.Anonymous1.dmPaperWidth = (paper_width / MM * 10.0).round() as i16;
            mode.Anonymous1.Anonymous1.dmPaperLength = (paper_height / MM * 10.0).round() as i16;
            if DocumentPropertiesW(
                null_mut(),
                handle.0,
                name.as_ptr(),
                mode_ptr,
                mode_ptr,
                DM_IN_BUFFER | DM_OUT_BUFFER,
            ) != 1
            {
                return Err(error("Validar tamaño de papel"));
            }
            Ok(())
        };
        apply_size(source.pages[0].width, source.pages[0].height)?;
        let raw = CreateDCW(driver.as_ptr(), name.as_ptr(), null(), mode_ptr);
        if raw.is_null() {
            return Err(error("CreateDCW"));
        }
        let mut dc = Dc(raw);
        let info = DOCINFOW {
            cbSize: size_of::<DOCINFOW>() as i32,
            lpszDocName: title.as_ptr(),
            lpszOutput: output.as_ref().map_or(null(), |p| p.as_ptr()),
            ..Default::default()
        };
        let job_id = StartDocW(dc.0, &info);
        if job_id <= 0 {
            return Err(error("StartDocW"));
        }
        let mut job = Job {
            dc: dc.0,
            completed: false,
        };
        for (index, page) in source
            .pages
            .iter()
            .cycle()
            .take(source.pages.len() * copies as usize)
            .enumerate()
        {
            if index > 0 {
                apply_size(page.width, page.height)?;
                let changed = ResetDCW(dc.0, mode_ptr);
                if changed.is_null() {
                    return Err(error("ResetDCW"));
                }
                dc.0 = changed;
                job.dc = changed;
            }
            let dpi_x = GetDeviceCaps(dc.0, LOGPIXELSX as i32) as f32;
            let dpi_y = GetDeviceCaps(dc.0, LOGPIXELSY as i32) as f32;
            ensure(
                dpi_x > 0.0 && dpi_y > 0.0,
                "PRINT_FAILED",
                "La impresora no informó una resolución válida.",
            )?;
            let usable_w = GetDeviceCaps(dc.0, HORZRES as i32) as f32 * 72.0 / dpi_x;
            let usable_h = GetDeviceCaps(dc.0, VERTRES as i32) as f32 * 72.0 / dpi_y;
            // Fit the usable content, never enlarge a thermal ticket onto A4.
            let content_w = page.width - 2.0 * page.margin;
            let content_h = page.height - 2.0 * page.margin + 8.0;
            let scale = (usable_w / content_w).min(usable_h / content_h).min(1.0);
            ensure(
                scale >= 0.85,
                "PRINTER_PAPER",
                "El papel configurado es demasiado pequeño. Revisa el ancho y tamaño en las propiedades de la impresora o guarda el PDF.",
            )?;
            let physical_x = GetDeviceCaps(dc.0, PHYSICALOFFSETX as i32) as f32 * 72.0 / dpi_x;
            let left = if source.kind == "labels" {
                page.margin - physical_x
            } else {
                (usable_w - content_w * scale) / 2.0
            };
            let physical_y = GetDeviceCaps(dc.0, PHYSICALOFFSETY as i32) as f32 * 72.0 / dpi_y;
            let top = if source.kind == "labels" {
                page.margin - physical_y
            } else {
                (page.margin - physical_y)
                    .max(0.0)
                    .min((usable_h - content_h * scale).max(0.0))
            };
            if source.kind == "labels" {
                let paper_w = GetDeviceCaps(dc.0, PHYSICALWIDTH as i32) as f32 * 72.0 / dpi_x;
                let paper_h = GetDeviceCaps(dc.0, PHYSICALHEIGHT as i32) as f32 * 72.0 / dpi_y;
                ensure(
                    scale >= 0.999
                        && left >= 0.0
                        && top >= 0.0
                        && (paper_w - page.width).abs() < MM
                        && (paper_h - page.height).abs() < MM,
                    "PRINTER_PAPER",
                    "Las etiquetas requieren papel A4 al 100%. Revisa el tamaño de papel o guarda el PDF para imprimirlo sin ajustar.",
                )?;
            }
            if StartPage(dc.0) <= 0 {
                return Err(error("StartPage"));
            }
            render_page(
                dc.0,
                page,
                &fonts,
                logo.as_ref(),
                PageTransform {
                    left,
                    top,
                    scale,
                    dpi_x,
                    dpi_y,
                    thermal_text: false,
                },
            )?;
            if EndPage(dc.0) <= 0 {
                return Err(error("EndPage"));
            }
        }
        if EndDoc(dc.0) <= 0 {
            return Err(error("EndDoc"));
        }
        job.completed = true;
        Ok(PrintJob{job_id,printer:printer.name.clone(),pages:source.pages.len()*copies as usize,message:"Windows recibió el documento. Puedes continuar; revisa la cola de impresión si no sale en papel.".into()})
    }
}
