use crate::{
    Engine, appearance, db,
    error::{AppError, Result, ensure, text},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rusqlite::params;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessInput {
    pub store_name: String,
    pub subtitle: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub tax_id: String,
    pub ticket_footer: String,
    pub folio_prefix: String,
    pub default_tax_basis_points: i64,
    pub ticket_width_mm: i64,
    pub printer_name: String,
    pub auto_print: bool,
    pub logo_action: String,
    pub logo_data: Option<String>,
}
impl Engine {
    pub fn save_business(&mut self, input: BusinessInput) -> Result<Value> {
        let actor = self.actor("admin")?;
        if input.auto_print {
            let printers = crate::documents::printing::printers()?;
            let printer = printers
                .iter()
                .find(|printer| printer.name == input.printer_name)
                .ok_or_else(|| {
                    AppError::rule(
                        "PRINTER_MISSING",
                        "Selecciona una impresora disponible para activar la impresión automática.",
                    )
                })?;
            ensure(
                !printer.virtual_printer,
                "PRINTER_VIRTUAL",
                "Para imprimir automáticamente selecciona una impresora física. Los documentos PDF se guardan desde la vista previa.",
            )?;
        }
        let name = text(&input.store_name, "Nombre del negocio", 150)?;
        let prefix = input.folio_prefix.trim().to_uppercase();
        ensure(
            !prefix.is_empty()
                && prefix.len() <= 8
                && prefix.chars().all(|c| c.is_ascii_alphanumeric()),
            "VALIDATION",
            "Utiliza un prefijo de 1 a 8 letras o números para los tickets.",
        )?;
        ensure(
            input.subtitle.len() <= 300
                && input.address.len() <= 500
                && input.phone.len() <= 60
                && input.email.len() <= 200
                && input.tax_id.len() <= 30
                && input.ticket_footer.len() <= 1500
                && input.printer_name.len() <= 512
                && (0..=10000).contains(&input.default_tax_basis_points)
                && [58, 80].contains(&input.ticket_width_mm),
            "VALIDATION",
            "Revisa los datos del negocio, el impuesto y el ancho del papel.",
        )?;
        ensure(
            ["keep", "replace", "remove"].contains(&input.logo_action.as_str()),
            "VALIDATION",
            "Selecciona una acción válida para el logotipo.",
        )?;
        let logo = if input.logo_action == "replace" {
            let raw = input.logo_data.as_deref().ok_or_else(|| {
                AppError::rule("INVALID_IMAGE", "Selecciona el logotipo del negocio.")
            })?;
            ensure(
                raw.len() <= 7 * 1024 * 1024,
                "INVALID_IMAGE",
                "Utiliza una imagen de hasta 5 MB.",
            )?;
            let bytes = STANDARD
                .decode(raw)
                .map_err(|_| AppError::rule("INVALID_IMAGE", "No pudimos leer el logotipo."))?;
            appearance::validate_image(&bytes)?;
            Some(bytes)
        } else {
            None
        };
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let id = db::scalar(&tx, "SELECT Id FROM StoreSettings LIMIT 1", [])?;
        tx.execute("UPDATE StoreSettings SET StoreName=?1,Address=?2,Phone=?3,Email=?4,TaxId=?5,TicketFooter=?6,FolioPrefix=?7,DefaultTaxBasisPoints=?8,TicketWidthMm=?9,PrinterName=?10 WHERE Id=?11", params![name,input.address.trim(),input.phone.trim(),input.email.trim(),input.tax_id.trim().to_uppercase(),input.ticket_footer.trim(),prefix,input.default_tax_basis_points,input.ticket_width_mm,input.printer_name,id])?;
        if input.logo_action != "keep" {
            tx.execute(
                "UPDATE StoreSettings SET LogoData=?1,LogoPath='' WHERE Id=?2",
                params![logo, id],
            )?;
        }
        tx.execute("INSERT INTO StorePresentation(StoreId,Subtitle,AutoPrint) VALUES (?1,?2,?3) ON CONFLICT(StoreId) DO UPDATE SET Subtitle=excluded.Subtitle,AutoPrint=excluded.AutoPrint", params![id,input.subtitle.trim(),input.auto_print])?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Configurar negocio y tickets",
            "Negocio",
            "",
        )?;
        tx.commit()?;
        self.bootstrap()
    }
}
