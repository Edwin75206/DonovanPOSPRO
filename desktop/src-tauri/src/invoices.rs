use crate::{
    Engine, database::Connection,
    error::{AppError, Result, ensure, text},
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceData {
    pub rfc: String,
    pub name: String,
    pub postal_code: String,
    pub tax_regime: String,
    pub cfdi_use: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InvoiceRequest {
    data: InvoiceData,
    requested_at: String,
    uuid: Option<String>,
}

fn key(id: i64) -> String {
    format!("sale-invoice-request:{id}")
}

fn load(db: &Connection, id: i64) -> Result<Option<InvoiceRequest>> {
    let stored: Option<String> = db.query_row(
        "SELECT (SELECT Value FROM AppMeta WHERE Key=?1)",
        [key(id)],
        |row| row.get(0),
    )?;
    Ok(stored.as_deref().map(serde_json::from_str).transpose()?)
}

fn store(db: &Connection, id: i64, request: &InvoiceRequest) -> Result<()> {
    db.execute(
        "INSERT INTO AppMeta(Key,Value) VALUES (?1,?2) ON CONFLICT(Key) DO UPDATE SET Value=excluded.Value",
        params![key(id), serde_json::to_string(request)?],
    )?;
    Ok(())
}

fn normalize(mut data: InvoiceData) -> Result<InvoiceData> {
    data.rfc = data.rfc.trim().to_uppercase();
    data.name = text(&data.name, "Nombre o razón social", 200)?;
    data.postal_code = data.postal_code.trim().into();
    data.tax_regime = data.tax_regime.trim().into();
    data.cfdi_use = data.cfdi_use.trim().to_ascii_uppercase();
    let rfc: Vec<char> = data.rfc.chars().collect();
    ensure(
        matches!(rfc.len(), 12 | 13),
        "VALIDATION",
        "El RFC debe tener 12 o 13 caracteres.",
    )?;
    let prefix = rfc.len() - 9;
    ensure(
        rfc[..prefix]
            .iter()
            .all(|&letter| matches!(letter, 'A'..='Z' | 'Ñ' | '&'))
            && rfc[prefix..prefix + 6].iter().all(char::is_ascii_digit)
            && rfc[prefix + 6..].iter().all(char::is_ascii_alphanumeric),
        "VALIDATION",
        "Comprueba el formato del RFC.",
    )?;
    ensure(
        data.postal_code.len() == 5 && data.postal_code.bytes().all(|digit| digit.is_ascii_digit()),
        "VALIDATION",
        "El código postal fiscal debe tener cinco dígitos.",
    )?;
    ensure(
        data.tax_regime.len() == 3 && data.tax_regime.bytes().all(|digit| digit.is_ascii_digit()),
        "VALIDATION",
        "Escribe la clave de tres dígitos del régimen fiscal.",
    )?;
    ensure(
        (3..=4).contains(&data.cfdi_use.len())
            && data.cfdi_use.bytes().all(|character| character.is_ascii_alphanumeric()),
        "VALIDATION",
        "Escribe la clave de uso del CFDI indicada por el cliente.",
    )?;
    Ok(data)
}

impl Engine {
    pub fn invoice_request(&self, id: i64) -> Result<Value> {
        self.sale(id)?;
        Ok(serde_json::to_value(load(self.db()?, id)?)?)
    }

    pub fn save_invoice_request(&mut self, id: i64, data: InvoiceData) -> Result<Value> {
        let detail = self.sale(id)?;
        ensure(
            detail["sale"]["cancelled"] != true,
            "VALIDATION",
            "La venta está cancelada.",
        )?;
        let data = normalize(data)?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let current = load(&tx, id)?;
        ensure(
            current.as_ref().is_none_or(|record| record.uuid.is_none()),
            "ALREADY_COMPLETED",
            "La solicitud ya tiene un folio fiscal registrado.",
        )?;
        let record = InvoiceRequest {
            data,
            requested_at: current.map_or(now, |record| record.requested_at),
            uuid: None,
        };
        store(&tx, id, &record)?;
        tx.commit()?;
        Ok(serde_json::to_value(record)?)
    }

    pub fn record_invoice_uuid(&mut self, id: i64, uuid: &str) -> Result<Value> {
        self.actor("admin")?;
        self.sale(id)?;
        let uuid = uuid.trim();
        let parsed = uuid::Uuid::parse_str(uuid).map_err(|_| {
            AppError::rule("VALIDATION", "Escribe el folio fiscal UUID del comprobante emitido.")
        })?;
        ensure(
            uuid.len() == 36 && !parsed.is_nil(),
            "VALIDATION",
            "Escribe el folio fiscal UUID del comprobante emitido.",
        )?;
        let uuid = parsed.hyphenated().to_string().to_ascii_uppercase();
        let tx = self.db_mut()?.transaction()?;
        let mut record = load(&tx, id)?.ok_or_else(|| {
            AppError::rule("NOT_FOUND", "Registra primero la solicitud de factura.")
        })?;
        ensure(
            record.uuid.as_ref().is_none_or(|existing| existing == &uuid),
            "ALREADY_COMPLETED",
            "La solicitud ya tiene un folio fiscal registrado.",
        )?;
        record.uuid = Some(uuid);
        store(&tx, id, &record)?;
        tx.commit()?;
        Ok(serde_json::to_value(record)?)
    }
}
