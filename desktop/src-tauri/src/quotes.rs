use crate::{
    Engine, catalog, commerce, db,
    error::{Result, ensure, text},
    models::SaleItem,
    operations,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteInput {
    pub request_id: String,
    pub id: i64,
    pub items: Vec<SaleItem>,
    pub customer_id: Option<i64>,
    pub discount_parts: i64,
    pub valid_until: String,
    pub notes: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeldInput {
    pub request_id: String,
    pub id: Option<i64>,
    pub label: String,
    pub items: Vec<SaleItem>,
    pub customer_id: Option<i64>,
    pub discount_parts: i64,
}
pub fn quote_detail(db: &Connection, id: i64) -> Result<Value> {
    Ok(
        json!({"quote":db::one(db,"SELECT * FROM Quote WHERE Id=?1",[id])?,"lines":db::all(db,"SELECT q.*,p.Sku,p.StockUnits,p.Active FROM QuoteLine q JOIN Product p ON p.Id=q.ProductId WHERE QuoteId=?1 ORDER BY q.Id",[id])?}),
    )
}
impl Engine {
    pub fn quotes(&self, search: &str) -> Result<Value> {
        self.actor("quote")?;
        ensure(search.len() <= 300, "VALIDATION", "Acorta la búsqueda.")?;
        Ok(json!(db::all(
            self.db()?,
            "SELECT * FROM Quote WHERE Folio LIKE '%'||?1||'%' OR CustomerName LIKE '%'||?1||'%' ORDER BY Id DESC LIMIT 1000",
            [search.trim()]
        )?))
    }
    pub fn quote(&self, id: i64) -> Result<Value> {
        self.actor("quote")?;
        quote_detail(self.db()?, id)
    }
    pub fn save_quote(&mut self, input: QuoteInput) -> Result<Value> {
        let actor = self.actor("quote")?;
        let hash = operations::fingerprint(&input)?;
        let now = self.now_string();
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(result) = operations::previous(&tx, &actor, &input.request_id, "quote", &hash)?
        {
            return Ok(result);
        }
        let valid = chrono::NaiveDate::parse_from_str(&input.valid_until, "%Y-%m-%d").ok();
        ensure(
            valid.is_some_and(|d| d.to_string() == input.valid_until)
                && input.valid_until.as_str() >= &now[..10]
                && input.notes.len() <= 2000,
            "VALIDATION",
            "Revisa la vigencia y utiliza observaciones de hasta 2000 caracteres.",
        )?;
        let charge = commerce::calculate(&tx, &actor, &input.items, input.discount_parts)?;
        let customer = if let Some(id) = input.customer_id {
            db::one(
                &tx,
                "SELECT Name FROM Customer WHERE Id=?1 AND Active=1",
                [id],
            )?["name"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        } else {
            "Público en general".into()
        };
        let (id, folio) = if input.id == 0 {
            let folio = db::folio(&tx, "quotes", "CT")?;
            tx.execute("INSERT INTO Quote(Folio,CreatedAt,ValidUntil,CustomerId,CustomerName,Notes,TotalCents,TaxCents,SaleId,UserId) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,NULL,?9)",params![folio,now,input.valid_until,input.customer_id,customer,input.notes.trim(),charge.total_cents,charge.tax_cents,actor.id])?;
            (tx.last_insert_rowid(), folio)
        } else {
            let old = db::one(&tx, "SELECT * FROM Quote WHERE Id=?1", [input.id])?;
            ensure(
                old["saleId"].is_null(),
                "ALREADY_COMPLETED",
                "La cotización ya se vendió y conserva su historial.",
            )?;
            tx.execute("UPDATE Quote SET ValidUntil=?1,CustomerId=?2,CustomerName=?3,Notes=?4,TotalCents=?5,TaxCents=?6 WHERE Id=?7",params![input.valid_until,input.customer_id,customer,input.notes.trim(),charge.total_cents,charge.tax_cents,input.id])?;
            tx.execute("DELETE FROM QuoteLine WHERE QuoteId=?1", [input.id])?;
            (input.id, old["folio"].as_str().unwrap_or("").to_owned())
        };
        for line in charge.lines {
            tx.execute("INSERT INTO QuoteLine(QuoteId,ProductId,Name,Unit,QuantityUnits,PriceCents,TaxBasisPoints,TotalCents) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![id,line.product_id,line.name,line.unit,line.quantity_units,line.price_cents,line.tax_basis_points,line.total_cents])?;
        }
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            if input.id == 0 {
                "Crear cotización"
            } else {
                "Editar cotización"
            },
            &folio,
            "",
        )?;
        let result = quote_detail(&tx, id)?;
        operations::receipt(
            &tx,
            &actor,
            &input.request_id,
            "quote",
            &hash,
            &result,
            &now,
        )?;
        tx.commit()?;
        Ok(result)
    }
    pub fn held_sales(&self) -> Result<Value> {
        let actor = self.actor("sell")?;
        Ok(json!(db::all(
            self.db()?,
            "SELECT h.*,u.Name AS UserName FROM HeldSale h JOIN User u ON u.Id=h.UserId WHERE h.SaleId IS NULL AND h.Discarded=0 AND (?1=0 OR h.UserId=?2) ORDER BY h.Id DESC LIMIT 1000",
            params![actor.role, actor.id]
        )?))
    }
    pub fn save_held(&mut self, input: HeldInput) -> Result<Value> {
        let actor = self.actor("sell")?;
        let label = text(&input.label, "Nombre de la venta", 100)?;
        let now = self.now_string();
        let hash = operations::fingerprint(&input)?;
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(result) = operations::previous(&tx, &actor, &input.request_id, "hold", &hash)? {
            return Ok(result);
        }
        let charge = commerce::calculate(&tx, &actor, &input.items, input.discount_parts)?;
        if let Some(id) = input.customer_id {
            db::one(
                &tx,
                "SELECT Id FROM Customer WHERE Id=?1 AND Active=1",
                [id],
            )?;
        }
        let id = if let Some(id) = input.id {
            let old = db::one(&tx, "SELECT * FROM HeldSale WHERE Id=?1", [id])?;
            ensure(
                old["saleId"].is_null() && old["discarded"] == false,
                "ALREADY_COMPLETED",
                "Esta venta suspendida ya se cerró.",
            )?;
            ensure(
                old["userId"] == actor.id || actor.role == 0,
                "FORBIDDEN",
                "Solo puedes cambiar tus ventas suspendidas.",
            )?;
            tx.execute(
                "UPDATE HeldSale SET Label=?1,CustomerId=?2,DiscountParts=?3 WHERE Id=?4",
                params![label, input.customer_id, input.discount_parts, id],
            )?;
            tx.execute("DELETE FROM HeldSaleLine WHERE HeldSaleId=?1", [id])?;
            id
        } else {
            let folio = db::folio(&tx, "held", "ESP")?;
            tx.execute("INSERT INTO HeldSale(Folio,Label,CreatedAt,UserId,CustomerId,SaleId,DiscountParts,Discarded) VALUES (?1,?2,?3,?4,?5,NULL,?6,0)",params![folio,label,now,actor.id,input.customer_id,input.discount_parts])?;
            tx.last_insert_rowid()
        };
        for (line, requested) in charge.lines.iter().zip(&input.items) {
            tx.execute("INSERT INTO HeldSaleLine(HeldSaleId,ProductId,QuantityUnits,Wholesale,OriginalPriceCents) VALUES (?1,?2,?3,?4,?5)",params![id,line.product_id,line.quantity_units,requested.wholesale,line.price_cents])?;
        }
        let result = db::one(&tx, "SELECT * FROM HeldSale WHERE Id=?1", [id])?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Suspender venta",
            result["folio"].as_str().unwrap_or(""),
            "",
        )?;
        operations::receipt(&tx, &actor, &input.request_id, "hold", &hash, &result, &now)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn held_sale(&self, id: i64) -> Result<Value> {
        let actor = self.actor("sell")?;
        let db = self.db()?;
        let held = db::one(db, "SELECT * FROM HeldSale WHERE Id=?1", [id])?;
        ensure(
            held["saleId"].is_null() && held["discarded"] == false,
            "ALREADY_COMPLETED",
            "Esta venta suspendida ya se cerró.",
        )?;
        ensure(
            actor.role == 0 || held["userId"] == actor.id,
            "FORBIDDEN",
            "Solo puedes recuperar tus ventas suspendidas.",
        )?;
        let mut lines = vec![];
        let mut notices = vec![];
        for row in db::all(
            db,
            "SELECT * FROM HeldSaleLine WHERE HeldSaleId=?1 ORDER BY Id",
            [id],
        )? {
            let mut p = catalog::product(db, row["productId"].as_i64().unwrap_or(0))?;
            let units = row["quantityUnits"].as_i64().unwrap_or(0);
            let wholesale = row["wholesale"] == true;
            if !p.active {
                notices.push(format!(
                    "{} está archivado. Retíralo antes de cobrar.",
                    p.name
                ));
            }
            if p.stock_units < units {
                notices.push(format!(
                    "{} tiene existencias insuficientes. Revisa la cantidad.",
                    p.name
                ));
            }
            if row["originalPriceCents"]
                != if wholesale {
                    p.wholesale_cents
                } else {
                    p.price_cents
                }
            {
                notices.push(format!("{} se actualizó al precio vigente.", p.name));
            }
            if wholesale && !actor.can("discount") {
                notices.push("El precio de mayoreo requiere autorización.".into());
            }
            if !actor.can("catalog") {
                p.cost_cents = None;
            }
            lines.push(json!({"product":p,"quantityUnits":units,"wholesale":wholesale}));
        }
        Ok(json!({"sale":held,"rows":lines,"notices":notices}))
    }
    pub fn discard_held(&mut self, id: i64) -> Result<Value> {
        let actor = self.actor("sell")?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let held = db::one(&tx, "SELECT * FROM HeldSale WHERE Id=?1", [id])?;
        ensure(
            actor.role == 0 || held["userId"] == actor.id,
            "FORBIDDEN",
            "Solo puedes descartar tus ventas suspendidas.",
        )?;
        ensure(
            held["saleId"].is_null(),
            "ALREADY_COMPLETED",
            "Esta venta ya se cobró.",
        )?;
        if held["discarded"] == false {
            tx.execute("UPDATE HeldSale SET Discarded=1 WHERE Id=?1", [id])?;
            db::audit(
                &tx,
                &now,
                Some(actor.id),
                "Descartar venta suspendida",
                held["folio"].as_str().unwrap_or(""),
                "",
            )?;
        }
        tx.commit()?;
        Ok(json!({"discarded":true}))
    }
}
