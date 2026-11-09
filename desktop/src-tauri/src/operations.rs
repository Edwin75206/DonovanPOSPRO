use crate::database::Connection;
use crate::{
    auth, db,
    error::{Result, ensure},
    models::Actor,
};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Value, json};
pub fn fingerprint(input: &impl Serialize) -> Result<String> {
    Ok(auth::digest(&serde_json::to_string(input)?))
}
pub fn previous(
    db: &Connection,
    actor: &Actor,
    id: &str,
    operation: &str,
    hash: &str,
) -> Result<Option<Value>> {
    ensure(
        uuid::Uuid::parse_str(id).is_ok(),
        "INVALID_REQUEST",
        "Abre nuevamente el formulario para obtener una referencia válida.",
    )?;
    let row = db::all(
        db,
        "SELECT Operation,ResultJson FROM RequestReceipt WHERE RequestId=?1",
        [id],
    )?
    .into_iter()
    .next();
    if let Some(row) = row {
        let saved: Value = serde_json::from_str(row["resultJson"].as_str().unwrap_or("{}"))?;
        ensure(
            row["operation"] == operation
                && saved["userId"] == actor.id
                && saved["inputHash"] == hash,
            "REQUEST_CHANGED",
            "Esta referencia ya se utilizó con otros datos. Consulta el historial antes de registrar otra operación.",
        )?;
        return Ok(Some(saved["result"].clone()));
    }
    Ok(None)
}
pub fn receipt(
    db: &Connection,
    actor: &Actor,
    id: &str,
    operation: &str,
    hash: &str,
    result: &Value,
    now: &str,
) -> Result<()> {
    db.execute(
        "INSERT INTO RequestReceipt(RequestId,Operation,ResultJson,CreatedAt) VALUES (?1,?2,?3,?4)",
        params![
            id,
            operation,
            serde_json::to_string(&json!({"userId":actor.id,"inputHash":hash,"result":result}))?,
            now
        ],
    )?;
    Ok(())
}
