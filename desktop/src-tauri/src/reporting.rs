use crate::database::Connection;
use crate::{
    Engine, commerce, db,
    error::{Result, ensure},
    money,
};
use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::params;
use serde_json::{Value, json};

pub fn range(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate)> {
    let first = NaiveDate::parse_from_str(from, "%Y-%m-%d").ok();
    let last = NaiveDate::parse_from_str(to, "%Y-%m-%d").ok();
    ensure(
        first.is_some() && last.is_some(),
        "VALIDATION",
        "Selecciona fechas válidas para consultar la información.",
    )?;
    let (first, last) = (first.unwrap(), last.unwrap());
    ensure(
        last >= first
            && (last - first).num_days() <= 366
            && first.year() >= 1900
            && last.year() <= 9998
            && from == first.to_string()
            && to == last.to_string(),
        "VALIDATION",
        "Selecciona un rango de hasta 367 días, con la fecha final después de la inicial.",
    )?;
    Ok((first, last + Duration::days(1)))
}
fn net(db: &Connection, from: &str, end: &str) -> Result<i64> {
    db::scalar(
        db,
        "SELECT (SELECT COALESCE(SUM(TotalCents),0) FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2)-(SELECT COALESCE(SUM(TotalCents),0) FROM SaleReturn WHERE CreatedAt>=?1 AND CreatedAt<?2)",
        params![from, end],
    )
}
fn number(value: &Value, key: &str) -> i64 {
    value[key].as_i64().unwrap_or(0)
}
const ACTIVITY_LINES: &str = "WITH activity AS (SELECT l.ProductId,l.Name,l.Category,l.Unit,l.QuantityUnits,l.TotalCents FROM SaleLine l JOIN Sale s ON s.Id=l.SaleId WHERE s.CreatedAt>=?1 AND s.CreatedAt<?2 UNION ALL SELECT l.ProductId,l.Name,l.Category,l.Unit,-rl.QuantityUnits,-rl.AmountCents FROM ReturnLine rl JOIN SaleReturn r ON r.Id=rl.SaleReturnId JOIN SaleLine l ON l.Id=rl.SaleLineId WHERE r.CreatedAt>=?1 AND r.CreatedAt<?2)";

impl Engine {
    pub fn dashboard(&self, from: &str, to: &str) -> Result<Value> {
        self.actor("reports")?;
        let (start, end) = range(from, to)?;
        let db = self.db()?;
        let end_text = end.to_string();
        let today = NaiveDate::parse_from_str(&self.now_string()[..10], "%Y-%m-%d").unwrap();
        let tomorrow = (today + Duration::days(1)).to_string();
        let previous_start = start - Duration::days((end - start).num_days());
        let gross = db::scalar(
            db,
            "SELECT COALESCE(SUM(TotalCents),0) FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2",
            params![from, end_text],
        )?;
        let returned = db::scalar(
            db,
            "SELECT COALESCE(SUM(TotalCents),0) FROM SaleReturn WHERE CreatedAt>=?1 AND CreatedAt<?2",
            params![from, end_text],
        )?;
        let raw_daily = db::all(
            db,
            "SELECT Day,SUM(Amount) AS TotalCents FROM (SELECT substr(CreatedAt,1,10) AS Day,TotalCents AS Amount FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2 UNION ALL SELECT substr(CreatedAt,1,10),-TotalCents FROM SaleReturn WHERE CreatedAt>=?1 AND CreatedAt<?2) AS activity GROUP BY Day",
            params![from, end_text],
        )?;
        let trend: Vec<Value> = (0..(end - start).num_days())
            .map(|i| {
                let date = (start + Duration::days(i)).to_string();
                let amount = raw_daily
                    .iter()
                    .find(|r| r["day"] == date)
                    .map(|r| number(r, "totalCents"))
                    .unwrap_or(0);
                json!({"date":date,"totalCents":amount})
            })
            .collect();
        let mut profit = Some(0i64);
        for line in db::all(
            db,
            "SELECT l.TotalCents,l.TaxCents,l.CostCents,l.QuantityUnits FROM SaleLine l JOIN Sale s ON s.Id=l.SaleId WHERE s.CreatedAt>=?1 AND s.CreatedAt<?2",
            params![from, end_text],
        )? {
            profit = match (profit, line["costCents"].as_i64()) {
                (Some(p), Some(c)) => Some(
                    p + number(&line, "totalCents")
                        - number(&line, "taxCents")
                        - money::line_total(c, number(&line, "quantityUnits"))?,
                ),
                _ => None,
            };
        }
        for line in db::all(
            db,
            "SELECT rl.AmountCents,rl.QuantityUnits,l.CostCents,l.TaxBasisPoints FROM ReturnLine rl JOIN SaleReturn r ON r.Id=rl.SaleReturnId JOIN SaleLine l ON l.Id=rl.SaleLineId WHERE r.CreatedAt>=?1 AND r.CreatedAt<?2",
            params![from, end_text],
        )? {
            profit = match (profit, line["costCents"].as_i64()) {
                (Some(p), Some(c)) => Some(
                    p - money::rounded(
                        number(&line, "amountCents") as i128 * 10000,
                        10000 + number(&line, "taxBasisPoints") as i128,
                    ) + money::line_total(c, number(&line, "quantityUnits"))?,
                ),
                _ => None,
            };
        }
        let open = db::all(db, "SELECT * FROM CashSession WHERE ClosedAt IS NULL", [])?
            .into_iter()
            .next();
        let cash = if let Some(id) = open.as_ref().and_then(|v| v["id"].as_i64()) {
            commerce::cash_balance(db, id)?
        } else {
            0
        };
        let methods = db::all(
            db,
            "SELECT Method,SUM(AmountCents) AS TotalCents FROM (SELECT p.Method,p.AmountCents FROM PaymentRecord p JOIN Sale s ON s.Id=p.SaleId WHERE s.CreatedAt>=?1 AND s.CreatedAt<?2 AND p.Method<3 UNION ALL SELECT Method,-AmountCents FROM CreditEntry WHERE Kind='Abono' AND CreatedAt>=?1 AND CreatedAt<?2 UNION ALL SELECT RefundMethod,-(TotalCents-CreditReductionCents) FROM SaleReturn WHERE CreatedAt>=?1 AND CreatedAt<?2) AS activity GROUP BY Method ORDER BY Method",
            params![from, end_text],
        )?;
        Ok(json!({
         "from":from,"to":to,"dayCents":net(db,&today.to_string(),&tomorrow)?,"weekCents":net(db,&(today-Duration::days(6)).to_string(),&tomorrow)?,"monthCents":net(db,&today.with_day(1).unwrap().to_string(),&tomorrow)?,
         "grossCents":gross,"returnsCents":returned,"totalCents":gross-returned,"previousCents":net(db,&previous_start.to_string(),from)?,"profitCents":profit,
         "operations":db::scalar(db,"SELECT COUNT(*) FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2",params![from,end_text])?,
         "dayOperations":db::scalar(db,"SELECT COUNT(*) FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2",params![today.to_string(),tomorrow])?,
         "receivableCents":db::scalar(db,"SELECT COALESCE(SUM(AmountCents),0) FROM CreditEntry",[])?,
         "creditGrantedCents":db::scalar(db,"SELECT COALESCE(SUM(AmountCents),0) FROM CreditEntry WHERE AmountCents>0 AND CreatedAt>=?1 AND CreatedAt<?2",params![from,end_text])?,
         "cashCents":cash,"cashSession":open,"lastCashClose":db::all(db,"SELECT * FROM CashSession WHERE ClosedAt IS NOT NULL ORDER BY Id DESC LIMIT 1",[])?.into_iter().next(),
         "trend":trend,"methods":methods,
         "topProducts":db::all(db,&format!("{ACTIVITY_LINES} SELECT ProductId,Name,Unit,SUM(QuantityUnits) AS QuantityUnits,SUM(TotalCents) AS TotalCents FROM activity GROUP BY ProductId,Name,Unit ORDER BY TotalCents DESC LIMIT 8"),params![from,end_text])?,
         "categories":db::all(db,&format!("{ACTIVITY_LINES} SELECT Category AS Name,SUM(TotalCents) AS TotalCents FROM activity GROUP BY Category ORDER BY TotalCents DESC"),params![from,end_text])?,
         "lowStock":db::all(db,"SELECT Id,Sku,Name,Unit,StockUnits,MinimumUnits FROM Product WHERE Active=1 AND StockUnits<=MinimumUnits ORDER BY StockUnits,Name LIMIT 10",[])?,
         "stockAlerts":db::one(db,"SELECT COALESCE(SUM(CASE WHEN StockUnits=0 THEN 1 ELSE 0 END),0) AS EmptyCount,COALESCE(SUM(CASE WHEN StockUnits>0 AND StockUnits<=MinimumUnits THEN 1 ELSE 0 END),0) AS LowCount FROM Product WHERE Active=1",[])?,
         "recentSales":db::all(db,"SELECT * FROM Sale ORDER BY Id DESC LIMIT 6",[])?,
         "recentPurchases":db::all(db,"SELECT p.*,s.Name AS SupplierName FROM Purchase p JOIN Supplier s ON s.Id=p.SupplierId ORDER BY p.Id DESC LIMIT 5",[])?,
         "activity":db::all(db,"SELECT a.*,u.Name AS UserName FROM AuditEvent a LEFT JOIN User u ON u.Id=a.UserId ORDER BY a.Id DESC LIMIT 8",[])?,
         "stockMovements":db::all(db,"SELECT m.*,p.Name AS ProductName,p.Unit FROM StockMovement m JOIN Product p ON p.Id=m.ProductId ORDER BY m.Id DESC LIMIT 6",[])?,
         "users":db::all(db,"SELECT u.Id,u.Name,u.Role,u.Active,(SELECT MAX(IssuedAt) FROM UserSession WHERE UserId=u.Id) AS LastAccess FROM User u ORDER BY u.Id",[])?,
         "lastBackup":db::all(db,"SELECT * FROM BackupRecord WHERE Successful=1 ORDER BY Id DESC LIMIT 1",[])?.into_iter().next(),
         "lastIntegrity":self.last_integrity,"databaseBytes":self.database_bytes()?,"schemaVersion":db::SCHEMA_VERSION,
        }))
    }
}
