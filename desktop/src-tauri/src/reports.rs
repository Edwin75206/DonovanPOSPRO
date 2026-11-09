//! Administrator reports. Money remains integer cents through aggregation.
//! Returns belong to their operation date, matching the executive dashboard.
use crate::database::Connection;
use crate::{
    Engine, db,
    error::{AppError, Result, ensure},
    money, reporting,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportQuery {
    pub kind: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub search: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportColumn {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub weight: f32,
}
#[derive(Debug, Serialize)]
pub struct ReportMetric {
    pub label: String,
    pub value: Option<i64>,
    pub kind: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub kind: String,
    pub title: String,
    pub scope: String,
    pub current: bool,
    pub from: String,
    pub to: String,
    pub search: String,
    pub generated_at: String,
    pub columns: Vec<ReportColumn>,
    pub rows: Vec<Value>,
    pub metrics: Vec<ReportMetric>,
    pub chart: Option<Value>,
}
fn col(key: &str, label: &str, kind: &str, weight: f32) -> ReportColumn {
    ReportColumn {
        key: key.into(),
        label: label.into(),
        kind: kind.into(),
        weight,
    }
}
fn num(row: &Value, key: &str) -> i64 {
    row[key].as_i64().unwrap_or(0)
}
fn folded(text: &str) -> String {
    text.nfkd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}
pub fn cell(value: &Value, column: &ReportColumn) -> String {
    if value.is_null() {
        return if column.key.to_lowercase().contains("cost") {
            "Sin costo"
        } else {
            "Pendiente"
        }
        .into();
    }
    match column.kind.as_str() {
        "money" => crate::documents::layout::money(value.as_i64().unwrap_or(0)),
        "quantity" => crate::documents::layout::quantity(value.as_i64().unwrap_or(0)),
        "unit" => crate::documents::layout::unit(value.as_i64().unwrap_or(0)).into(),
        "date" => crate::documents::date(value.as_str().unwrap_or("")),
        _ => value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string()),
    }
}
fn sum(rows: &[Value], key: &str, label: &str) -> ReportMetric {
    let value = rows.iter().try_fold(0i64, |total, row| {
        row[key].as_i64().and_then(|n| total.checked_add(n))
    });
    ReportMetric {
        label: label.into(),
        value,
        kind: "money".into(),
    }
}
fn count(value: i64, label: &str) -> ReportMetric {
    ReportMetric {
        label: label.into(),
        value: Some(value),
        kind: "integer".into(),
    }
}
const LINES: &str = "WITH activity AS (
 SELECT l.ProductId,l.Sku,l.Name,l.Category,l.Unit,l.QuantityUnits,l.TotalCents AS GrossCents,0 AS ReturnCents FROM SaleLine l JOIN Sale s ON s.Id=l.SaleId WHERE s.CreatedAt>=?1 AND s.CreatedAt<?2
 UNION ALL SELECT l.ProductId,l.Sku,l.Name,l.Category,l.Unit,-rl.QuantityUnits,0,rl.AmountCents FROM ReturnLine rl JOIN SaleReturn r ON r.Id=rl.SaleReturnId JOIN SaleLine l ON l.Id=rl.SaleLineId WHERE r.CreatedAt>=?1 AND r.CreatedAt<?2)";
const LIMIT_MESSAGE: &str = "El reporte supera 50,000 filas. Reduce el periodo o utiliza una selección más pequeña antes de exportar.";

fn profit_rows(db: &Connection, from: &str, end: &str) -> Result<Vec<Value>> {
    // Stream source lines, then group. No SQLite floating point multiplication.
    let stmt = db.prepare("SELECT l.ProductId,l.Sku,l.Name,l.Unit,l.QuantityUnits,l.TotalCents-l.TaxCents AS NetCents,l.CostCents,1 AS Direction,l.TaxBasisPoints FROM SaleLine l JOIN Sale s ON s.Id=l.SaleId WHERE s.CreatedAt>=?1 AND s.CreatedAt<?2
 UNION ALL SELECT l.ProductId,l.Sku,l.Name,l.Unit,rl.QuantityUnits,rl.AmountCents,l.CostCents,-1,l.TaxBasisPoints FROM ReturnLine rl JOIN SaleReturn r ON r.Id=rl.SaleReturnId JOIN SaleLine l ON l.Id=rl.SaleLineId WHERE r.CreatedAt>=?1 AND r.CreatedAt<?2 LIMIT 250001")?;
    let mut cursor = stmt.query(params![from, end])?;
    let mut groups: BTreeMap<(i64, String, String, i64), (i64, i64, Option<i64>)> = BTreeMap::new();
    let mut scanned = 0;
    while let Some(row) = cursor.next()? {
        scanned += 1;
        ensure(
            scanned <= 250000,
            "REPORT_LIMIT",
            "El cálculo supera 250,000 partidas. Selecciona un periodo más corto para calcular las ganancias.",
        )?;
        let key = (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?);
        let qty: i64 = row.get(4)?;
        let amount: i64 = row.get(5)?;
        let cost: Option<i64> = row.get(6)?;
        let sign: i64 = row.get(7)?;
        let tax: i64 = row.get(8)?;
        let net = if sign == 1 {
            amount
        } else {
            money::rounded(amount as i128 * 10000, 10000 + tax as i128)
        };
        let line_cost = cost.map(|c| money::line_total(c, qty)).transpose()?;
        let values = groups.entry(key).or_insert((0, 0, Some(0)));
        values.0 += sign * qty;
        values.1 += sign * net;
        values.2 = values.2.zip(line_cost).map(|(total, c)| total + sign * c);
    }
    let mut rows:Vec<Value>=groups.into_iter().map(|((_,sku,name,unit),(qty,net,cost))| json!({"sku":sku,"name":name,"unit":unit,"quantityUnits":qty,"netCents":net,"costCents":cost,"profitCents":cost.map(|c|net-c)})).collect();
    rows.sort_by_key(|row| std::cmp::Reverse(num(row, "netCents")));
    Ok(rows)
}
impl Engine {
    pub fn report(&self, query: &ReportQuery) -> Result<Report> {
        self.actor("reports")?;
        let (start, end_date) = reporting::range(&query.from, &query.to)?;
        ensure(
            query.search.chars().count() <= 120,
            "VALIDATION",
            "Usa hasta 120 caracteres para buscar en el reporte.",
        )?;
        let from = start.to_string();
        let end = end_date.to_string();
        let tx = self.db()?.unchecked_transaction()?;
        let query_rows = |sql: &str| -> Result<Vec<Value>> {
            db::all(&tx, &format!("{sql} LIMIT 50001"), params![from, end])
        };
        let t = |key, label, weight| col(key, label, "text", weight);
        let m = |key, label, weight| col(key, label, "money", weight);
        let d = |key, label, weight| col(key, label, "date", weight);
        let q = |key, label, weight| col(key, label, "quantity", weight);
        let u = || col("unit", "Unidad", "unit", 0.65);
        let i = |key, label, weight| col(key, label, "integer", weight);
        let (title, scope, columns, mut rows, chart_keys): (
            &str,
            &str,
            Vec<ReportColumn>,
            Vec<Value>,
            Option<(&str, &str, &str)>,
        ) = match query.kind.as_str() {
            "daily" => {
                let raw = query_rows(
                    "SELECT Day AS Date,SUM(GrossCents) AS GrossCents,SUM(ReturnCents) AS ReturnCents,SUM(GrossCents)-SUM(ReturnCents) AS NetCents,SUM(Operations) AS Operations FROM (SELECT substr(CreatedAt,1,10) AS Day,TotalCents AS GrossCents,0 AS ReturnCents,1 AS Operations FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2 UNION ALL SELECT substr(CreatedAt,1,10),0,TotalCents,0 FROM SaleReturn WHERE CreatedAt>=?1 AND CreatedAt<?2) AS activity GROUP BY Day ORDER BY Day",
                )?;
                let days=(0..(end_date-start).num_days()).map(|n| {
                    let date=(start+chrono::Duration::days(n)).to_string();
                    raw.iter().find(|r|r["date"]==date).cloned().unwrap_or_else(||json!({"date":date,"grossCents":0,"returnCents":0,"netCents":0,"operations":0}))
                }).collect();
                (
                    "Ventas por día",
                    "Ventas y devoluciones por su fecha de operación. Las ventas netas incluyen IVA; el crédito se cuenta como venta, aunque todavía no se haya cobrado.",
                    vec![
                        d("date", "Fecha", 1.1),
                        i("operations", "Ventas", 0.8),
                        m("grossCents", "Venta original", 1.2),
                        m("returnCents", "Devoluciones", 1.2),
                        m("netCents", "Ventas netas", 1.2),
                    ],
                    days,
                    Some(("date", "netCents", "Ventas netas")),
                )
            }
            "sales" => (
                "Ventas registradas",
                "Ventas emitidas en el periodo, con su importe original. Consulta las devoluciones por su propia fecha en el reporte Devoluciones.",
                vec![
                    t("folio", "Folio", 0.9),
                    d("createdAt", "Fecha", 1.2),
                    t("customerName", "Cliente", 1.7),
                    t("cashier", "Usuario", 1.4),
                    m("totalCents", "Total original", 1.1),
                    m("taxCents", "IVA", 1.0),
                    t("state", "Estado actual", 1.1),
                ],
                query_rows(
                    "SELECT Folio,CreatedAt,CustomerName,Cashier,TotalCents,TaxCents,CASE WHEN Cancelled=1 THEN 'Cancelada / devuelta' ELSE 'Registrada' END AS State FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2 ORDER BY CreatedAt DESC,Id DESC",
                )?,
                None,
            ),
            "products" => (
                "Ventas por producto",
                "Importes netos con IVA. Las devoluciones se descuentan en la fecha en que se registraron; las cantidades se mantienen en la unidad de cada producto.",
                vec![
                    t("sku", "Clave", 0.8),
                    t("name", "Producto", 2.2),
                    q("quantityUnits", "Cantidad neta", 1.0),
                    u(),
                    m("grossCents", "Venta original", 1.1),
                    m("returnCents", "Devuelto", 1.1),
                    m("netCents", "Venta neta", 1.1),
                ],
                query_rows(&format!(
                    "{LINES} SELECT Sku,Name,Unit,SUM(QuantityUnits) AS QuantityUnits,SUM(GrossCents) AS GrossCents,SUM(ReturnCents) AS ReturnCents,SUM(GrossCents)-SUM(ReturnCents) AS NetCents FROM activity GROUP BY ProductId,Sku,Name,Unit ORDER BY NetCents DESC,Name"
                ))?,
                Some(("name", "netCents", "Productos por venta neta")),
            ),
            "categories" => (
                "Ventas por categoría",
                "Ventas y devoluciones por fecha de operación, con las categorías guardadas en cada venta. Importes con IVA.",
                vec![
                    t("name", "Categoría", 2.0),
                    m("grossCents", "Venta original", 1.2),
                    m("returnCents", "Devuelto", 1.2),
                    m("netCents", "Venta neta", 1.2),
                ],
                query_rows(&format!(
                    "{LINES} SELECT COALESCE(NULLIF(Category,''),'Sin categoría') AS Name,SUM(GrossCents) AS GrossCents,SUM(ReturnCents) AS ReturnCents,SUM(GrossCents)-SUM(ReturnCents) AS NetCents FROM activity GROUP BY Category ORDER BY NetCents DESC,Name"
                ))?,
                Some(("name", "netCents", "Ventas por categoría")),
            ),
            "users" => (
                "Ventas por usuario",
                "Las devoluciones se descuentan en su fecha de operación y se atribuyen al usuario de la venta original. Importes con IVA.",
                vec![
                    t("name", "Usuario", 2.0),
                    i("operations", "Ventas emitidas", 1.0),
                    m("grossCents", "Venta original", 1.2),
                    m("returnCents", "Devuelto", 1.2),
                    m("netCents", "Venta neta", 1.2),
                ],
                query_rows(
                    "SELECT Cashier AS Name,SUM(Operations) AS Operations,SUM(GrossCents) AS GrossCents,SUM(ReturnCents) AS ReturnCents,SUM(GrossCents)-SUM(ReturnCents) AS NetCents FROM (SELECT UserId,Cashier,1 AS Operations,TotalCents AS GrossCents,0 AS ReturnCents FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<?2 UNION ALL SELECT s.UserId,s.Cashier,0,0,r.TotalCents FROM SaleReturn r JOIN Sale s ON s.Id=r.SaleId WHERE r.CreatedAt>=?1 AND r.CreatedAt<?2) AS activity GROUP BY UserId,Cashier ORDER BY NetCents DESC,Name",
                )?,
                Some(("name", "netCents", "Ventas netas por usuario")),
            ),
            "methods" => (
                "Cobros por método",
                "Cobros de ventas y abonos recibidos, menos reembolsos, por su fecha de operación. El crédito pendiente no se considera dinero recibido.",
                vec![
                    t("name", "Método", 1.8),
                    m("collectedCents", "Cobros y abonos", 1.2),
                    m("refundCents", "Reembolsos", 1.2),
                    m("netCents", "Cobro neto", 1.2),
                ],
                query_rows(
                    "SELECT CASE Method WHEN 0 THEN 'Efectivo' WHEN 1 THEN 'Tarjeta' WHEN 2 THEN 'Transferencia' END AS Name,SUM(CollectedCents) AS CollectedCents,SUM(RefundCents) AS RefundCents,SUM(CollectedCents)-SUM(RefundCents) AS NetCents FROM (SELECT p.Method,p.AmountCents AS CollectedCents,0 AS RefundCents FROM PaymentRecord p JOIN Sale s ON s.Id=p.SaleId WHERE s.CreatedAt>=?1 AND s.CreatedAt<?2 AND p.Method<3 UNION ALL SELECT Method,-AmountCents,0 FROM CreditEntry WHERE Kind='Abono' AND CreatedAt>=?1 AND CreatedAt<?2 UNION ALL SELECT RefundMethod,0,TotalCents-CreditReductionCents FROM SaleReturn WHERE CreatedAt>=?1 AND CreatedAt<?2) AS activity WHERE Method BETWEEN 0 AND 2 GROUP BY Method ORDER BY Method",
                )?,
                Some(("name", "netCents", "Cobros netos")),
            ),
            "profit" => (
                "Costos y ganancias",
                "Estimación sin IVA y antes de gastos, basada en el costo guardado al vender. Las devoluciones afectan su fecha de operación. Un costo faltante deja pendiente la ganancia; no se sustituye por cero.",
                vec![
                    t("sku", "Clave", 0.8),
                    t("name", "Producto", 2.1),
                    q("quantityUnits", "Cantidad neta", 1.0),
                    u(),
                    m("netCents", "Venta sin IVA", 1.2),
                    m("costCents", "Costo estimado", 1.2),
                    m("profitCents", "Ganancia estimada", 1.3),
                ],
                profit_rows(&tx, &from, &end)?,
                Some(("name", "profitCents", "Ganancias calculables")),
            ),
            "inventory" => (
                "Inventario actual",
                "Existencias y precios actuales, incluidos productos archivados. No es una reconstrucción histórica. La valuación utiliza el costo neto actual; un costo faltante queda pendiente.",
                vec![
                    t("sku", "Clave", 0.8),
                    t("name", "Producto", 2.0),
                    q("stockUnits", "Existencia", 0.9),
                    u(),
                    q("minimumUnits", "Mínimo", 0.8),
                    m("costCents", "Costo neto", 1.0),
                    m("priceCents", "Precio con IVA", 1.0),
                    m("valuationCents", "Valor a costo", 1.1),
                    t("state", "Estado", 1.0),
                ],
                db::all(
                    &tx,
                    "SELECT Sku,Name,Unit,StockUnits,MinimumUnits,CostCents,PriceCents,CASE WHEN Active=0 THEN 'Archivado' WHEN StockUnits=0 THEN 'Agotado' WHEN StockUnits<=MinimumUnits THEN 'Stock bajo' ELSE 'Disponible' END AS State FROM Product ORDER BY Name LIMIT 50001",
                    [],
                )?,
                Some(("name", "valuationCents", "Valor del inventario")),
            ),
            "purchases" => (
                "Compras",
                "Recepciones registradas en el periodo. Se conserva el costo exacto de la compra, incluidos empaques. Los registros históricos sin forma de pago se identifican expresamente.",
                vec![
                    t("folio", "Folio", 0.9),
                    d("createdAt", "Fecha", 1.2),
                    t("supplierName", "Proveedor", 2.0),
                    t("supplierReference", "Referencia", 1.5),
                    m("totalCents", "Costo neto", 1.1),
                    t("payment", "Pago desde caja", 1.0),
                ],
                query_rows(
                    "SELECT p.Folio,p.CreatedAt,s.Name AS SupplierName,p.SupplierReference,p.TotalCents,CASE WHEN d.PaidFromCash=1 THEN 'Sí' WHEN d.PaidFromCash=0 THEN 'No' ELSE 'Sin dato histórico' END AS Payment FROM Purchase p JOIN Supplier s ON s.Id=p.SupplierId LEFT JOIN PurchaseDetails d ON d.PurchaseId=p.Id WHERE p.CreatedAt>=?1 AND p.CreatedAt<?2 ORDER BY p.CreatedAt DESC,p.Id DESC",
                )?,
                None,
            ),
            "receivables" => (
                "Cuentas por cobrar",
                "Saldo actual por cliente, incluyendo cuentas archivadas con deuda. Los abonos y las reducciones por devolución ya están descontados. El periodo no modifica este saldo actual.",
                vec![
                    t("name", "Cliente", 2.2),
                    t("phone", "Teléfono", 1.3),
                    m("balanceCents", "Saldo pendiente", 1.3),
                    t("state", "Cuenta", 1.0),
                ],
                db::all(
                    &tx,
                    "SELECT c.Name,c.Phone,SUM(e.AmountCents) AS BalanceCents,CASE WHEN c.Active=1 THEN 'Activa' ELSE 'Archivada' END AS State FROM Customer c JOIN CreditEntry e ON e.CustomerId=c.Id GROUP BY c.Id HAVING SUM(e.AmountCents)<>0 ORDER BY BalanceCents DESC,c.Name LIMIT 50001",
                    [],
                )?,
                Some(("name", "balanceCents", "Saldos por cliente")),
            ),
            "credits" => (
                "Movimientos de crédito",
                "Movimientos registrados en el periodo. Un cargo aumenta la deuda; los abonos y las devoluciones la reducen. Este movimiento neto no representa por sí solo el saldo actual.",
                vec![
                    d("createdAt", "Fecha", 1.2),
                    t("customerName", "Cliente", 1.7),
                    t("folio", "Venta", 0.9),
                    t("kind", "Movimiento", 1.2),
                    m("amountCents", "Cambio en deuda", 1.2),
                    t("reference", "Referencia", 1.4),
                    t("userName", "Usuario", 1.3),
                ],
                query_rows(
                    "SELECT e.CreatedAt,c.Name AS CustomerName,s.Folio,e.Kind,e.AmountCents,e.Reference,u.Name AS UserName FROM CreditEntry e JOIN Customer c ON c.Id=e.CustomerId JOIN Sale s ON s.Id=e.SaleId JOIN User u ON u.Id=e.UserId WHERE e.CreatedAt>=?1 AND e.CreatedAt<?2 ORDER BY e.CreatedAt DESC,e.Id DESC",
                )?,
                None,
            ),
            "returns" => (
                "Devoluciones",
                "Devoluciones por su fecha de registro. El importe se divide entre reducción de crédito y dinero reembolsado, sin descontar dos veces la venta.",
                vec![
                    t("folio", "Folio", 0.8),
                    d("createdAt", "Fecha", 1.1),
                    t("saleFolio", "Venta", 0.8),
                    m("totalCents", "Total devuelto", 1.0),
                    m("creditReductionCents", "Crédito reducido", 1.0),
                    m("paidCents", "Reembolsado", 1.0),
                    t("method", "Método", 1.0),
                    t("reason", "Motivo", 2.0),
                ],
                query_rows(
                    "SELECT r.Folio,r.CreatedAt,s.Folio AS SaleFolio,r.TotalCents,r.CreditReductionCents,r.TotalCents-r.CreditReductionCents AS PaidCents,CASE r.RefundMethod WHEN 0 THEN 'Efectivo' WHEN 1 THEN 'Tarjeta' WHEN 2 THEN 'Transferencia' ELSE 'Sin reembolso' END AS Method,r.Reason FROM SaleReturn r JOIN Sale s ON s.Id=r.SaleId WHERE r.CreatedAt>=?1 AND r.CreatedAt<?2 ORDER BY r.CreatedAt DESC,r.Id DESC",
                )?,
                None,
            ),
            "cash" => (
                "Jornadas de caja",
                "Jornadas cuya apertura está dentro del periodo. La caja abierta muestra su efectivo esperado actual; las diferencias solo se calculan para cierres con conteo.",
                vec![
                    t("folio", "Folio", 0.9),
                    d("openedAt", "Apertura", 1.3),
                    d("closedAt", "Cierre", 1.3),
                    t("userName", "Responsable", 1.5),
                    m("openingCents", "Fondo inicial", 1.1),
                    m("expectedCents", "Esperado", 1.1),
                    m("countedCents", "Contado", 1.1),
                    m("differenceCents", "Diferencia", 1.1),
                ],
                query_rows(
                    "SELECT c.Folio,c.OpenedAt,c.ClosedAt,u.Name AS UserName,c.OpeningCents,COALESCE(c.ExpectedCents,c.OpeningCents+(SELECT COALESCE(SUM(AmountCents),0) FROM CashMovement WHERE CashSessionId=c.Id)) AS ExpectedCents,c.CountedCents,CASE WHEN c.ClosedAt IS NOT NULL THEN c.CountedCents-c.ExpectedCents END AS DifferenceCents FROM CashSession c JOIN User u ON u.Id=c.UserId WHERE c.OpenedAt>=?1 AND c.OpenedAt<?2 ORDER BY c.OpenedAt DESC,c.Id DESC",
                )?,
                None,
            ),
            "cashMovements" => (
                "Movimientos de caja",
                "Entradas y salidas de efectivo por su fecha de operación. No incluye el fondo de apertura como ingreso ni pagos por tarjeta o transferencia.",
                vec![
                    d("createdAt", "Fecha", 1.2),
                    t("folio", "Caja", 0.9),
                    t("kind", "Movimiento", 1.2),
                    m("amountCents", "Efectivo", 1.1),
                    t("reference", "Referencia", 1.1),
                    t("reason", "Motivo", 1.8),
                    t("userName", "Usuario", 1.3),
                ],
                query_rows(
                    "SELECT m.CreatedAt,c.Folio,m.Kind,m.AmountCents,m.Reference,m.Reason,u.Name AS UserName FROM CashMovement m JOIN CashSession c ON c.Id=m.CashSessionId JOIN User u ON u.Id=m.UserId WHERE m.CreatedAt>=?1 AND m.CreatedAt<?2 ORDER BY m.CreatedAt DESC,m.Id DESC",
                )?,
                None,
            ),
            _ => {
                return Err(AppError::rule(
                    "VALIDATION",
                    "Selecciona un reporte disponible.",
                ));
            }
        };
        ensure(rows.len() <= 50000, "REPORT_LIMIT", LIMIT_MESSAGE)?;
        if query.kind == "inventory" {
            for row in &mut rows {
                row["valuationCents"] = if num(row, "stockUnits") == 0 {
                    json!(0)
                } else {
                    row["costCents"]
                        .as_i64()
                        .map(|c| money::line_total(c, num(row, "stockUnits")))
                        .transpose()?
                        .map(Value::from)
                        .unwrap_or(Value::Null)
                };
            }
        }
        let search = folded(query.search.trim());
        if !search.is_empty() {
            rows.retain(|row| {
                columns.iter().any(|column| {
                    folded(&cell(&row[&column.key], column)).contains(&search)
                        || folded(&row[&column.key].to_string()).contains(&search)
                })
            });
        }
        let metrics = match query.kind.as_str() {
            "daily" | "products" | "categories" | "users" => vec![
                sum(&rows, "grossCents", "Venta original"),
                sum(&rows, "returnCents", "Devoluciones"),
                sum(&rows, "netCents", "Ventas netas"),
            ],
            "sales" => vec![
                count(rows.len() as i64, "Ventas emitidas"),
                sum(&rows, "totalCents", "Venta original"),
                sum(&rows, "taxCents", "IVA de las ventas"),
            ],
            "methods" => vec![
                sum(&rows, "collectedCents", "Cobros y abonos"),
                sum(&rows, "refundCents", "Reembolsos"),
                sum(&rows, "netCents", "Cobros netos"),
            ],
            "profit" => vec![
                sum(&rows, "netCents", "Ventas sin IVA"),
                sum(&rows, "costCents", "Costo estimado"),
                sum(&rows, "profitCents", "Ganancia estimada"),
                count(
                    rows.iter().filter(|r| r["costCents"].is_null()).count() as i64,
                    "Productos sin costo",
                ),
            ],
            "inventory" => vec![
                count(rows.len() as i64, "Productos"),
                sum(&rows, "valuationCents", "Valor total a costo"),
                count(
                    rows.iter().filter(|r| r["costCents"].is_null()).count() as i64,
                    "Productos sin costo",
                ),
            ],
            "purchases" => vec![
                count(rows.len() as i64, "Compras recibidas"),
                sum(&rows, "totalCents", "Costo neto total"),
            ],
            "receivables" => vec![
                count(rows.len() as i64, "Clientes con saldo"),
                sum(&rows, "balanceCents", "Pendiente por cobrar"),
            ],
            "returns" => vec![
                sum(&rows, "totalCents", "Total devuelto"),
                sum(&rows, "creditReductionCents", "Crédito reducido"),
                sum(&rows, "paidCents", "Dinero reembolsado"),
            ],
            "cash" => vec![
                count(rows.len() as i64, "Jornadas"),
                count(
                    rows.iter().filter(|r| !r["closedAt"].is_null()).count() as i64,
                    "Jornadas cerradas",
                ),
                sum(
                    &rows
                        .iter()
                        .filter(|r| !r["closedAt"].is_null())
                        .cloned()
                        .collect::<Vec<_>>(),
                    "differenceCents",
                    "Diferencia de cierres",
                ),
            ],
            _ => vec![
                count(rows.len() as i64, "Movimientos"),
                sum(
                    &rows,
                    "amountCents",
                    if query.kind == "credits" {
                        "Cambio neto en deuda"
                    } else {
                        "Movimiento neto de efectivo"
                    },
                ),
            ],
        };
        let chart = chart_keys.map(|(label, key, title)| {
            let mut values: Vec<Value> = rows
                .iter()
                .filter_map(|row| {
                    row[key]
                        .as_i64()
                        .map(|n| json!({"name":row[label],"valueCents":n}))
                })
                .collect();
            if query.kind != "daily" {
                values.sort_by_key(|v| std::cmp::Reverse(num(v, "valueCents")));
                values.truncate(12);
            }
            json!({"title":title,"values":values,"daily":query.kind=="daily"})
        });
        for (index, row) in rows.iter_mut().enumerate() {
            row["id"] = json!(index);
        }
        tx.commit()?;
        Ok(Report {
            kind: query.kind.clone(),
            title: title.into(),
            scope: scope.into(),
            current: matches!(query.kind.as_str(), "inventory" | "receivables"),
            from: query.from.clone(),
            to: query.to.clone(),
            search: query.search.trim().into(),
            generated_at: self.now_string(),
            columns,
            rows,
            metrics,
            chart,
        })
    }
}
