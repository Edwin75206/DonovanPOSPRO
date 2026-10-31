use crate::{
    Engine, db,
    error::{Result, ensure},
};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesQuery {
    pub from: String,
    pub to: String,
    pub search: String,
    pub method: Option<i64>,
    pub status: String,
    pub page: i64,
    #[serde(default)]
    pub sort: String,
    #[serde(default = "default_descending")]
    pub descending: bool,
}
fn default_descending() -> bool {
    true
}
impl Engine {
    pub fn sales_page(&self, query: SalesQuery) -> Result<Value> {
        let actor = self.actor("sell")?;
        for value in [&query.from, &query.to] {
            ensure(
                chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .is_ok_and(|date| date.to_string() == *value),
                "VALIDATION",
                "Selecciona fechas válidas para consultar las ventas.",
            )?;
        }
        ensure(
            query.from <= query.to
                && query.search.len() <= 300
                && (0..=10_000_000).contains(&query.page)
                && query.method.is_none_or(|method| (0..=3).contains(&method))
                && ["all", "active", "refunded"].contains(&query.status.as_str()),
            "VALIDATION",
            "Revisa las fechas y filtros de la consulta.",
        )?;
        let sort = match query.sort.as_str() {
            "" | "folio" => "s.Id",
            "customer" => "s.CustomerName COLLATE NOCASE",
            "amount" => "s.TotalCents",
            "refund" => "RefundedCents",
            "state" => {
                "CASE WHEN s.Cancelled=1 THEN 2 WHEN EXISTS(SELECT 1 FROM SaleReturn WHERE SaleId=s.Id) THEN 1 ELSE 0 END"
            }
            _ => {
                return Err(crate::error::AppError::rule(
                    "VALIDATION",
                    "Selecciona una columna válida para ordenar las ventas.",
                ));
            }
        };
        let direction = if query.descending { "DESC" } else { "ASC" };
        let snapshot = self.db()?.unchecked_transaction()?;
        let db = &snapshot;
        let end = format!("{} 23:59:59.9999999", query.to);
        let predicate = "s.CreatedAt>=?1 AND s.CreatedAt<=?2 AND (?3=0 OR s.UserId=?4) AND (s.Folio LIKE '%'||?5||'%' OR s.CustomerName LIKE '%'||?5||'%' OR s.Cashier LIKE '%'||?5||'%') AND (?6 IS NULL OR EXISTS(SELECT 1 FROM PaymentRecord WHERE SaleId=s.Id AND Method=?6)) AND (?7='all' OR (?7='active' AND s.Cancelled=0) OR (?7='refunded' AND EXISTS(SELECT 1 FROM SaleReturn WHERE SaleId=s.Id)))";
        let summary = db::one(
            db,
            &format!(
                "SELECT COUNT(*) AS Count,COALESCE(SUM(s.TotalCents),0) AS TotalCents,COALESCE(SUM((SELECT COALESCE(SUM(TotalCents),0) FROM SaleReturn WHERE SaleId=s.Id)),0) AS RefundedCents FROM Sale s WHERE {predicate}"
            ),
            params![
                query.from,
                end,
                actor.role,
                actor.id,
                query.search.trim(),
                query.method,
                query.status
            ],
        )?;
        let items = db::all(
            db,
            &format!(
                "SELECT s.*,COALESCE((SELECT SUM(TotalCents) FROM SaleReturn WHERE SaleId=s.Id),0) AS RefundedCents FROM Sale s WHERE {predicate} ORDER BY {sort} {direction},s.Id DESC LIMIT 60 OFFSET ?8"
            ),
            params![
                query.from,
                end,
                actor.role,
                actor.id,
                query.search.trim(),
                query.method,
                query.status,
                query.page * 60
            ],
        )?;
        snapshot.commit()?;
        Ok(
            json!({"items":items,"total":summary["count"],"totalCents":summary["totalCents"],"refundedCents":summary["refundedCents"],"page":query.page,"pageSize":60}),
        )
    }
}
