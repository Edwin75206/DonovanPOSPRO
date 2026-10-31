use crate::{
    Engine, commerce, db,
    error::{Result, ensure, text},
    money, operations,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Contact {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub credit_limit_cents: i64,
    pub active: bool,
}
impl Default for Contact {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            phone: String::new(),
            email: String::new(),
            address: String::new(),
            credit_limit_cents: 0,
            active: true,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditPayment {
    pub request_id: String,
    pub customer_id: i64,
    pub sale_id: i64,
    pub amount_cents: i64,
    pub method: i64,
    pub reference: String,
}
impl Engine {
    pub fn contact(&self, supplier: bool, id: i64) -> Result<Value> {
        self.actor(if supplier { "purchase" } else { "sell" })?;
        let table = if supplier { "Supplier" } else { "Customer" };
        let balance = if supplier {
            "0"
        } else {
            "COALESCE((SELECT SUM(AmountCents) FROM CreditEntry WHERE CustomerId=c.Id),0)"
        };
        db::one(
            self.db()?,
            &format!("SELECT c.*,{balance} AS BalanceCents FROM {table} c WHERE Id=?1"),
            [id],
        )
    }
    pub fn contacts(&self, supplier: bool, search: &str, archived: bool) -> Result<Value> {
        self.actor(if supplier { "purchase" } else { "sell" })?;
        ensure(search.len() <= 300, "VALIDATION", "Acorta la búsqueda.")?;
        let table = if supplier { "Supplier" } else { "Customer" };
        let balance = if supplier {
            "0"
        } else {
            "COALESCE((SELECT SUM(AmountCents) FROM CreditEntry WHERE CustomerId=c.Id),0)"
        };
        Ok(json!(db::all(
            self.db()?,
            &format!(
                "SELECT c.*,{balance} AS BalanceCents FROM {table} c WHERE Active=?1 AND (Name LIKE '%'||?2||'%' COLLATE NOCASE OR Phone LIKE '%'||?2||'%') ORDER BY Name LIMIT 1000"
            ),
            params![!archived, search.trim()]
        )?))
    }
    pub fn save_contact(&mut self, supplier: bool, mut input: Contact) -> Result<Value> {
        let actor = self.actor(if supplier { "purchase" } else { "credit" })?;
        input.name = text(&input.name, "Nombre", 150)?;
        ensure(
            input.id >= 0
                && input.phone.len() <= 60
                && input.email.len() <= 200
                && input.address.len() <= 500
                && (0..=money::MAX_AMOUNT).contains(&input.credit_limit_cents),
            "VALIDATION",
            "Revisa los datos de contacto y el límite de crédito.",
        )?;
        let table = if supplier { "Supplier" } else { "Customer" };
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let id = if input.id == 0 {
            tx.execute(&format!("INSERT INTO {table}(Name,Phone,Email,Address,Active{}) VALUES (?1,?2,?3,?4,?5{})",if supplier{""}else{",CreditLimitCents"},if supplier{""}else{",0"}),params![input.name,input.phone.trim(),input.email.trim(),input.address.trim(),input.active])?;
            tx.last_insert_rowid()
        } else {
            ensure(tx.execute(&format!("UPDATE {table} SET Name=?1,Phone=?2,Email=?3,Address=?4,Active=?5 WHERE Id=?6"),params![input.name,input.phone.trim(),input.email.trim(),input.address.trim(),input.active,input.id])?==1,"NOT_FOUND","No encontramos este contacto.")?;
            input.id
        };
        if !supplier {
            tx.execute(
                "UPDATE Customer SET CreditLimitCents=?1 WHERE Id=?2",
                params![input.credit_limit_cents, id],
            )?;
        }
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            if supplier {
                "Guardar proveedor"
            } else {
                "Guardar cliente"
            },
            &id.to_string(),
            "",
        )?;
        let result = db::one(&tx, &format!("SELECT * FROM {table} WHERE Id=?1"), [id])?;
        tx.commit()?;
        Ok(result)
    }
    pub fn customer_ledger(&self, id: i64) -> Result<Value> {
        self.actor("credit")?;
        let db = self.db()?;
        Ok(
            json!({"customer":db::one(db,"SELECT * FROM Customer WHERE Id=?1",[id])?,"balanceCents":commerce::credit_balance(db,id,None)?,"entries":db::all(db,"SELECT e.*,s.Folio AS SaleFolio,u.Name AS UserName FROM CreditEntry e JOIN Sale s ON s.Id=e.SaleId JOIN User u ON u.Id=e.UserId WHERE e.CustomerId=?1 ORDER BY e.Id DESC",[id])?,"sales":db::all(db,"SELECT s.*,COALESCE((SELECT SUM(AmountCents) FROM CreditEntry WHERE SaleId=s.Id),0) AS BalanceCents,(SELECT MIN(DueDate) FROM CreditEntry WHERE SaleId=s.Id AND AmountCents>0) AS DueDate FROM Sale s WHERE s.CustomerId=?1 ORDER BY s.Id DESC LIMIT 1000",[id])?}),
        )
    }
    pub fn credit_payment(&mut self, input: CreditPayment) -> Result<Value> {
        let actor = self.actor("credit")?;
        let hash = operations::fingerprint(&input)?;
        let now = self.now_string();
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(result) =
            operations::previous(&tx, &actor, &input.request_id, "credit-payment", &hash)?
        {
            return Ok(result);
        }
        ensure(
            (1..=money::MAX_AMOUNT).contains(&input.amount_cents)
                && (0..=2).contains(&input.method)
                && input.reference.len() <= 200,
            "VALIDATION",
            "Indica un abono positivo y un método de pago válido.",
        )?;
        let sale = db::one(&tx, "SELECT * FROM Sale WHERE Id=?1", [input.sale_id])?;
        ensure(
            sale["customerId"] == input.customer_id,
            "VALIDATION",
            "La venta pendiente no corresponde a este cliente.",
        )?;
        ensure(
            input.amount_cents
                <= commerce::credit_balance(&tx, input.customer_id, Some(input.sale_id))?,
            "PAYMENT_EXCEEDS_DEBT",
            "El abono supera el saldo pendiente de esta venta.",
        )?;
        let cash = commerce::open_cash(&tx)?;
        let folio = db::folio(&tx, "credit-payment", "AB")?;
        let reference = if input.reference.trim().is_empty() {
            folio.clone()
        } else {
            format!("{} · {}", folio, input.reference.trim())
        };
        tx.execute("INSERT INTO CreditEntry(CustomerId,SaleId,CreatedAt,DueDate,AmountCents,Method,Kind,Reference,UserId) VALUES (?1,?2,?3,NULL,?4,?5,'Abono',?6,?7)",params![input.customer_id,input.sale_id,now,-input.amount_cents,input.method,reference,actor.id])?;
        let id = tx.last_insert_rowid();
        if input.method == 0 {
            commerce::cash_move(
                &tx,
                &now,
                actor.id,
                cash,
                input.amount_cents,
                "Abono",
                &folio,
                "",
            )?;
        }
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Registrar abono",
            &folio,
            sale["folio"].as_str().unwrap_or(""),
        )?;
        let result = json!({"id":id,"folio":folio,"amountCents":input.amount_cents,"balanceCents":commerce::credit_balance(&tx,input.customer_id,Some(input.sale_id))?});
        operations::receipt(
            &tx,
            &actor,
            &input.request_id,
            "credit-payment",
            &hash,
            &result,
            &now,
        )?;
        tx.commit()?;
        Ok(result)
    }
}
