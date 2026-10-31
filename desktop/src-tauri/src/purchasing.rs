use crate::{
    Engine, catalog, commerce, db,
    error::{Result, ensure, text},
    money, operations,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseItem {
    pub product_id: i64,
    pub quantity_units: i64,
    pub unit_cost_cents: i64,
    pub by_pack: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseInput {
    pub request_id: String,
    pub supplier_id: i64,
    pub reference: String,
    pub date: String,
    pub items: Vec<PurchaseItem>,
    pub paid_from_cash: bool,
}
impl Engine {
    pub fn purchases(&self, search: &str) -> Result<Value> {
        self.actor("purchase")?;
        ensure(search.len() <= 300, "VALIDATION", "Acorta la búsqueda.")?;
        Ok(json!(db::all(
            self.db()?,
            "SELECT p.*,s.Name AS SupplierName,u.Name AS UserName FROM Purchase p JOIN Supplier s ON s.Id=p.SupplierId JOIN User u ON u.Id=p.UserId WHERE p.Folio LIKE '%'||?1||'%' OR p.SupplierReference LIKE '%'||?1||'%' OR s.Name LIKE '%'||?1||'%' ORDER BY p.Id DESC LIMIT 1000",
            [search.trim()]
        )?))
    }
    pub fn purchase(&self, id: i64) -> Result<Value> {
        self.actor("purchase")?;
        let db = self.db()?;
        Ok(
            json!({"purchase":db::one(db,"SELECT p.*,s.Name AS SupplierName FROM Purchase p JOIN Supplier s ON s.Id=p.SupplierId WHERE p.Id=?1",[id])?,"lines":db::all(db,"SELECT l.*,p.Name,p.Sku,p.Unit FROM PurchaseLine l JOIN Product p ON p.Id=l.ProductId WHERE l.PurchaseId=?1 ORDER BY l.Id",[id])?}),
        )
    }
    pub fn receive_purchase(&mut self, input: PurchaseInput) -> Result<Value> {
        let actor = self.actor("purchase")?;
        let reference = text(&input.reference, "Folio del proveedor", 100)?;
        let hash = operations::fingerprint(&input)?;
        let now = self.now_string();
        let date = chrono::NaiveDate::parse_from_str(&input.date, "%Y-%m-%d").ok();
        ensure(
            date.is_some_and(|d| d.to_string() == input.date) && input.date.as_str() <= &now[..10],
            "VALIDATION",
            "Indica una fecha válida, igual o anterior a hoy.",
        )?;
        ensure(
            !input.items.is_empty() && input.items.len() <= 500,
            "VALIDATION",
            "Agrega entre 1 y 500 partidas a la compra.",
        )?;
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(result) =
            operations::previous(&tx, &actor, &input.request_id, "purchase", &hash)?
        {
            return Ok(result);
        }
        db::one(
            &tx,
            "SELECT Id FROM Supplier WHERE Id=?1 AND Active=1",
            [input.supplier_id],
        )?;
        ensure(
            db::scalar(
                &tx,
                "SELECT COUNT(*) FROM Purchase WHERE SupplierId=?1 AND SupplierReference=?2 COLLATE NOCASE",
                params![input.supplier_id, reference],
            )? == 0,
            "DUPLICATE_PURCHASE",
            "Este folio del proveedor ya fue recibido. Consulta el historial de compras.",
        )?;
        let folio = db::folio(&tx, "purchases", "CP")?;
        let created_at = format!("{} {}", input.date, &now[11..]);
        tx.execute("INSERT INTO Purchase(SupplierId,Folio,SupplierReference,CreatedAt,UserId,TotalCents) VALUES (?1,?2,?3,?4,?5,0)",params![input.supplier_id,folio,reference,created_at,actor.id])?;
        let id = tx.last_insert_rowid();
        let mut total = 0;
        for item in &input.items {
            let p = catalog::product(&tx, item.product_id)?;
            ensure(
                p.active,
                "PRODUCT_UNAVAILABLE",
                "Activa el producto antes de recibir nuevas existencias.",
            )?;
            ensure(
                (1..=money::MAX_UNITS).contains(&item.quantity_units)
                    && (0..=money::MAX_AMOUNT).contains(&item.unit_cost_cents),
                "VALIDATION",
                "Revisa las cantidades y costos de la compra.",
            )?;
            let (units, cost) = if item.by_pack {
                ensure(
                    item.quantity_units % 10000 == 0 && p.pack_units > 0,
                    "VALIDATION",
                    "Las presentaciones de compra se reciben completas.",
                )?;
                let units = item.quantity_units as i128 * p.pack_units as i128 / 10000;
                ensure(
                    units <= money::MAX_UNITS as i128,
                    "VALIDATION",
                    "La cantidad de la presentación supera el límite permitido.",
                )?;
                (
                    units as i64,
                    money::rounded(item.unit_cost_cents as i128 * 10000, p.pack_units as i128),
                )
            } else {
                (item.quantity_units, item.unit_cost_cents)
            };
            money::quantity(p.unit, units)?;
            ensure(
                cost <= money::MAX_AMOUNT,
                "VALIDATION",
                "El costo por unidad supera el límite permitido.",
            )?;
            let line_total = money::line_total(item.unit_cost_cents, item.quantity_units)?;
            total += line_total;
            ensure(
                total <= money::MAX_AMOUNT,
                "VALIDATION",
                "El total supera el límite permitido.",
            )?;
            let average = if let Some(previous) = p.cost_cents.filter(|_| p.stock_units > 0) {
                money::rounded(
                    previous as i128 * p.stock_units as i128 + cost as i128 * units as i128,
                    p.stock_units as i128 + units as i128,
                )
            } else {
                cost
            };
            catalog::stock(&tx, &now, actor.id, p.id, units, "Compra", &folio, "")?;
            tx.execute(
                "UPDATE Product SET CostCents=?1 WHERE Id=?2",
                params![average, p.id],
            )?;
            tx.execute("INSERT INTO PurchaseLine(PurchaseId,ProductId,QuantityUnits,CostCents) VALUES (?1,?2,?3,?4)",params![id,p.id,units,cost])?;
        }
        tx.execute(
            "UPDATE Purchase SET TotalCents=?1 WHERE Id=?2",
            params![total, id],
        )?;
        if input.paid_from_cash {
            let cash = commerce::open_cash(&tx)?;
            commerce::cash_move(&tx, &now, actor.id, cash, -total, "Compra", &folio, "")?;
        }
        db::audit(&tx, &now, Some(actor.id), "Recibir compra", &folio, "")?;
        let result = db::one(&tx, "SELECT * FROM Purchase WHERE Id=?1", [id])?;
        operations::receipt(
            &tx,
            &actor,
            &input.request_id,
            "purchase",
            &hash,
            &result,
            &now,
        )?;
        tx.commit()?;
        Ok(result)
    }
}
