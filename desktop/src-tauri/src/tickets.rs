//! Device-independent ticket preferences; the Windows adapter owns driver interaction.
use crate::{
    Engine, db,
    error::{Result, ensure},
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketConfig {
    pub printer_name: String,
    pub width_mm: i64,
    pub margin_mm: i64,
    pub copies: i64,
    pub monochrome: bool,
    pub show_logo: bool,
    pub auto_print: bool,
    pub footer: String,
    pub legend: String,
}
impl TicketConfig {
    pub fn validate(&self) -> Result<()> {
        ensure(
            [58, 80].contains(&self.width_mm)
                && (2..=8).contains(&self.margin_mm)
                && (1..=3).contains(&self.copies)
                && self.printer_name.len() <= 512
                && self.footer.len() <= 1500
                && self.legend.len() <= 500,
            "VALIDATION",
            "Revisa el ancho, márgenes (2 a 8 mm), copias (1 a 3) y textos del ticket.",
        )
    }
    pub fn load(db: &Connection) -> Result<Self> {
        let value = db::one(
            db,
            "SELECT s.PrinterName,s.TicketWidthMm AS WidthMm,s.TicketFooter AS Footer,COALESCE(p.AutoPrint,0) AS AutoPrint,COALESCE(t.MarginMm,4) AS MarginMm,COALESCE(t.Copies,1) AS Copies,COALESCE(t.Monochrome,1) AS Monochrome,COALESCE(t.ShowLogo,1) AS ShowLogo,COALESCE(t.Legend,'Conserva este comprobante.') AS Legend FROM StoreSettings s LEFT JOIN StorePresentation p ON p.StoreId=s.Id LEFT JOIN TicketProfile t ON t.StoreId=s.Id LIMIT 1",
            [],
        )?;
        // Computed SQLite columns have no declared BOOLEAN type.
        Ok(Self {
            printer_name: value["printerName"].as_str().unwrap_or("").into(),
            width_mm: value["widthMm"].as_i64().unwrap_or(80),
            margin_mm: value["marginMm"].as_i64().unwrap_or(4),
            copies: value["copies"].as_i64().unwrap_or(1),
            monochrome: truth(&value["monochrome"]),
            show_logo: truth(&value["showLogo"]),
            auto_print: truth(&value["autoPrint"]),
            footer: value["footer"].as_str().unwrap_or("").into(),
            legend: value["legend"].as_str().unwrap_or("").into(),
        })
    }
}
fn truth(v: &Value) -> bool {
    v.as_bool().unwrap_or_else(|| v.as_i64() == Some(1))
}
impl Engine {
    pub fn ticket_config(&self) -> Result<TicketConfig> {
        self.actor("sell")?;
        TicketConfig::load(self.db()?)
    }
    pub fn save_ticket_config(&mut self, input: TicketConfig) -> Result<Value> {
        let actor = self.actor("admin")?;
        input.validate()?;
        if input.auto_print {
            let available = crate::documents::printing::printers()?;
            ensure(
                available
                    .iter()
                    .any(|p| p.name == input.printer_name && !p.virtual_printer),
                "PRINTER_MISSING",
                "Selecciona una impresora física instalada para activar la impresión automática.",
            )?;
        }
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let id = db::scalar(&tx, "SELECT Id FROM StoreSettings LIMIT 1", [])?;
        tx.execute(
            "UPDATE StoreSettings SET PrinterName=?1,TicketWidthMm=?2,TicketFooter=?3 WHERE Id=?4",
            params![input.printer_name, input.width_mm, input.footer.trim(), id],
        )?;
        tx.execute("INSERT INTO StorePresentation(StoreId,Subtitle,AutoPrint) VALUES(?1,'',?2) ON CONFLICT(StoreId) DO UPDATE SET AutoPrint=excluded.AutoPrint",params![id,input.auto_print])?;
        tx.execute("INSERT INTO TicketProfile(StoreId,MarginMm,Copies,Monochrome,ShowLogo,Legend) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(StoreId) DO UPDATE SET MarginMm=excluded.MarginMm,Copies=excluded.Copies,Monochrome=excluded.Monochrome,ShowLogo=excluded.ShowLogo,Legend=excluded.Legend",params![id,input.margin_mm,input.copies,input.monochrome,input.show_logo,input.legend.trim()])?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Configurar impresoras y tickets",
            "Negocio",
            "",
        )?;
        tx.commit()?;
        self.bootstrap()
    }
}
