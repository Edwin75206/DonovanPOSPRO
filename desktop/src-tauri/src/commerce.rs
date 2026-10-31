use crate::{
    catalog, db,
    engine::Engine,
    error::{Result, ensure, text},
    models::{Actor, Charge, ChargeLine, Checkout, SaleItem},
    money, operations,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

pub fn calculate(
    db: &Connection,
    actor: &Actor,
    items: &[SaleItem],
    discount: i64,
) -> Result<Charge> {
    ensure(
        !items.is_empty() && items.len() <= 500,
        "EMPTY_CART",
        "Agrega entre 1 y 500 partidas antes de continuar.",
    )?;
    ensure(
        (0..=1_000_000).contains(&discount),
        "VALIDATION",
        "El descuento debe estar entre 0 y 100%.",
    )?;
    ensure(
        discount == 0 && !items.iter().any(|x| x.wholesale) || actor.can("discount"),
        "FORBIDDEN",
        "Los descuentos y precios de mayoreo requieren autorización.",
    )?;
    let mut seen = HashSet::new();
    let mut lines = vec![];
    for item in items {
        ensure(
            seen.insert(item.product_id),
            "VALIDATION",
            "Agrupa las cantidades del mismo producto en una sola partida.",
        )?;
        let p = catalog::product(db, item.product_id)?;
        ensure(
            p.active,
            "PRODUCT_UNAVAILABLE",
            "Un producto está archivado. Retíralo antes de continuar.",
        )?;
        money::quantity(p.unit, item.quantity_units)?;
        let price = if item.wholesale {
            p.wholesale_cents
        } else {
            p.price_cents
        };
        lines.push(ChargeLine {
            product_id: p.id,
            sku: p.sku,
            name: p.name,
            category: p.category,
            unit: p.unit,
            quantity_units: item.quantity_units,
            price_cents: price,
            cost_cents: p.cost_cents,
            tax_basis_points: p.tax_basis_points,
            total_cents: money::line_total(price, item.quantity_units)?,
            tax_cents: 0,
        });
    }
    let before: i64 = lines.iter().map(|x| x.total_cents).sum();
    ensure(
        before <= money::MAX_AMOUNT,
        "VALIDATION",
        "El total supera el límite permitido.",
    )?;
    let total = money::rounded(before as i128 * (1_000_000 - discount) as i128, 1_000_000);
    let allocation = money::allocate(
        total,
        &lines.iter().map(|x| x.total_cents).collect::<Vec<_>>(),
    )?;
    for (line, amount) in lines.iter_mut().zip(allocation) {
        line.total_cents = amount;
    }
    let tax = tax(&mut lines)?;
    Ok(Charge {
        lines,
        total_cents: total,
        tax_cents: tax,
        discount_cents: before - total,
    })
}
pub fn tax(lines: &mut [ChargeLine]) -> Result<i64> {
    let mut groups: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (i, line) in lines.iter().enumerate() {
        groups.entry(line.tax_basis_points).or_default().push(i);
    }
    for (rate, indexes) in groups {
        let weights: Vec<i64> = indexes.iter().map(|&i| lines[i].total_cents).collect();
        let gross: i64 = weights.iter().sum();
        let vat = gross - money::rounded(gross as i128 * 10000, 10000 + rate as i128);
        for (i, amount) in indexes.iter().zip(money::allocate(vat, &weights)?) {
            lines[*i].tax_cents = amount;
        }
    }
    Ok(lines.iter().map(|l| l.tax_cents).sum())
}
pub fn open_cash(db: &Connection) -> Result<i64> {
    let rows = db::all(db, "SELECT Id FROM CashSession WHERE ClosedAt IS NULL", [])?;
    ensure(
        rows.len() == 1,
        "CASH_CLOSED",
        "Abre la caja e indica el fondo inicial antes de registrar esta operación.",
    )?;
    Ok(rows[0]["id"].as_i64().unwrap_or(0))
}
pub fn cash_balance(db: &Connection, id: i64) -> Result<i64> {
    db::scalar(
        db,
        "SELECT OpeningCents+COALESCE((SELECT SUM(AmountCents) FROM CashMovement WHERE CashSessionId=c.Id),0) FROM CashSession c WHERE c.Id=?1",
        [id],
    )
}
pub fn cash_move(
    db: &Connection,
    now: &str,
    user: i64,
    id: i64,
    cents: i64,
    kind: &str,
    reference: &str,
    reason: &str,
) -> Result<()> {
    if cents == 0 {
        return Ok(());
    }
    ensure(
        cash_balance(db, id)? + cents >= 0,
        "INSUFFICIENT_CASH",
        "No hay suficiente efectivo en caja para completar esta operación.",
    )?;
    db.execute("INSERT INTO CashMovement(CashSessionId,UserId,CreatedAt,AmountCents,Kind,Reference,Reason) VALUES (?1,?2,?3,?4,?5,?6,?7)",params![id,user,now,cents,kind,reference,reason])?;
    Ok(())
}
pub fn credit_balance(db: &Connection, customer: i64, sale: Option<i64>) -> Result<i64> {
    db::scalar(
        db,
        "SELECT COALESCE(SUM(AmountCents),0) FROM CreditEntry WHERE CustomerId=?1 AND (?2 IS NULL OR SaleId=?2)",
        params![customer, sale],
    )
}
pub fn sale_detail(db: &Connection, actor: &Actor, id: i64) -> Result<Value> {
    let sale = db::one(db, "SELECT * FROM Sale WHERE Id=?1", [id])?;
    ensure(
        actor.role == 0 || sale["userId"] == actor.id,
        "FORBIDDEN",
        "Solo puedes consultar y reimprimir tus propias ventas.",
    )?;
    let mut lines = db::all(
        db,
        "SELECT * FROM SaleLine WHERE SaleId=?1 ORDER BY Id",
        [id],
    )?;
    if !actor.can("catalog") {
        for line in &mut lines {
            line["costCents"] = Value::Null;
        }
    }
    Ok(
        json!({"sale":sale,"lines":lines,"payments":db::all(db,"SELECT * FROM PaymentRecord WHERE SaleId=?1",[id])?,"returns":db::all(db,"SELECT * FROM SaleReturn WHERE SaleId=?1",[id])?}),
    )
}
impl Engine {
    pub fn price(&self, items: &[SaleItem], discount: i64) -> Result<Charge> {
        let actor = self.actor("sell")?;
        let mut charge = calculate(self.db()?, &actor, items, discount)?;
        if !actor.can("catalog") {
            for line in &mut charge.lines {
                line.cost_cents = None;
            }
        }
        Ok(charge)
    }
    pub fn cash_overview(&self) -> Result<Value> {
        self.actor("cash")?;
        let open = db::all(
            self.db()?,
            "SELECT * FROM CashSession WHERE ClosedAt IS NULL",
            [],
        )?
        .into_iter()
        .next();
        let id = open.as_ref().and_then(|v| v["id"].as_i64()).unwrap_or(0);
        Ok(
            json!({"open":open,"expectedCents":if id>0{cash_balance(self.db()?,id)?}else{0},"movements":db::all(self.db()?,"SELECT * FROM CashMovement WHERE CashSessionId=?1 ORDER BY Id DESC LIMIT 1000",[id])?,"history":db::all(self.db()?,"SELECT * FROM CashSession WHERE ClosedAt IS NOT NULL ORDER BY Id DESC LIMIT 200",[])?}),
        )
    }
    pub fn cash_open(&mut self, opening: i64) -> Result<Value> {
        let actor = self.actor("cash")?;
        ensure(
            (0..=money::MAX_AMOUNT).contains(&opening),
            "VALIDATION",
            "El fondo inicial debe ser un importe positivo o cero.",
        )?;
        let now = self.now_string();
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        ensure(
            db::scalar(
                &tx,
                "SELECT COUNT(*) FROM CashSession WHERE ClosedAt IS NULL",
                [],
            )? == 0,
            "CASH_ALREADY_OPEN",
            "Ya hay una caja abierta.",
        )?;
        let folio = db::folio(&tx, "cash", "CJ")?;
        tx.execute("INSERT INTO CashSession(Folio,UserId,OpenedAt,ClosedAt,OpeningCents,ExpectedCents,CountedCents,Notes) VALUES (?1,?2,?3,NULL,?4,NULL,NULL,'')",params![folio,actor.id,now,opening])?;
        db::audit(&tx, &now, Some(actor.id), "Abrir caja", &folio, "")?;
        tx.commit()?;
        self.cash_overview()
    }
    pub fn cash_movement(&mut self, cents: i64, reason: &str) -> Result<Value> {
        let actor = self.actor("cash")?;
        let reason = text(reason, "Motivo", 300)?;
        ensure(
            cents != 0 && (-money::MAX_AMOUNT..=money::MAX_AMOUNT).contains(&cents),
            "VALIDATION",
            "Indica un importe válido distinto de cero.",
        )?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let id = open_cash(&tx)?;
        cash_move(
            &tx,
            &now,
            actor.id,
            id,
            cents,
            if cents > 0 { "Ingreso" } else { "Egreso" },
            "Manual",
            &reason,
        )?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Movimiento de caja",
            &id.to_string(),
            &reason,
        )?;
        tx.commit()?;
        self.cash_overview()
    }
    pub fn cash_close(&mut self, counted: i64, notes: &str) -> Result<Value> {
        let actor = self.actor("cash")?;
        ensure(
            (0..=money::MAX_AMOUNT).contains(&counted) && notes.len() <= 1000,
            "VALIDATION",
            "Revisa el efectivo contado y las notas.",
        )?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        let id = open_cash(&tx)?;
        let expected = cash_balance(&tx, id)?;
        tx.execute("UPDATE CashSession SET ClosedAt=?1,ExpectedCents=?2,CountedCents=?3,Notes=?4 WHERE Id=?5",params![now,expected,counted,notes,id])?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Cerrar caja",
            &id.to_string(),
            &format!("Diferencia: {} centavos", counted - expected),
        )?;
        tx.commit()?;
        self.cash_overview()
    }
    pub fn checkout(&mut self, request: &Checkout) -> Result<Value> {
        let actor = self.actor("sell")?;
        ensure(
            uuid::Uuid::parse_str(&request.request_id).is_ok(),
            "INVALID_REQUEST",
            "Inicia una venta nueva para obtener una referencia válida.",
        )?;
        let now = self.now_string();
        let today = now[..10].to_owned();
        let hash = operations::fingerprint(request)?;
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        operations::previous(&tx, &actor, &request.request_id, "checkout", &hash)?;
        let existing = db::all(
            &tx,
            "SELECT Id FROM Sale WHERE RequestId=?1",
            [&request.request_id],
        )?;
        if let Some(existing) = existing.first() {
            return sale_detail(&tx, &actor, existing["id"].as_i64().unwrap_or(0));
        }
        let cash = open_cash(&tx)?;
        if let Some(id) = request.held_sale_id {
            let held = db::one(&tx, "SELECT * FROM HeldSale WHERE Id=?1", [id])?;
            ensure(
                held["saleId"].is_null() && held["discarded"] == false,
                "ALREADY_COMPLETED",
                "Esta venta suspendida ya se cerró.",
            )?;
            ensure(
                held["userId"] == actor.id || actor.role == 0,
                "FORBIDDEN",
                "Solo puedes recuperar tus ventas suspendidas.",
            )?;
            ensure(
                request.quote_id.is_none(),
                "VALIDATION",
                "Inicia por separado la venta de la cotización.",
            )?;
        }
        let mut customer_id = request.customer_id;
        let charge = if let Some(id) = request.quote_id {
            let quote = db::one(&tx, "SELECT * FROM Quote WHERE Id=?1", [id])?;
            ensure(
                quote["saleId"].is_null(),
                "ALREADY_COMPLETED",
                "Esta cotización ya se convirtió en venta.",
            )?;
            ensure(
                quote["validUntil"]
                    .as_str()
                    .unwrap_or("")
                    .get(..10)
                    .unwrap_or("")
                    >= today.as_str(),
                "QUOTE_EXPIRED",
                "La cotización venció. Revisa su vigencia antes de vender.",
            )?;
            if customer_id.is_none() {
                customer_id = quote["customerId"].as_i64();
            }
            let mut lines = vec![];
            for row in db::all(
                &tx,
                "SELECT * FROM QuoteLine WHERE QuoteId=?1 ORDER BY Id",
                [id],
            )? {
                let p = catalog::product(&tx, row["productId"].as_i64().unwrap_or(0))?;
                ensure(
                    p.active,
                    "PRODUCT_UNAVAILABLE",
                    "La cotización contiene un producto archivado.",
                )?;
                lines.push(ChargeLine {
                    product_id: p.id,
                    sku: p.sku,
                    name: row["name"].as_str().unwrap_or("").into(),
                    category: p.category,
                    unit: row["unit"].as_i64().unwrap_or(0),
                    quantity_units: row["quantityUnits"].as_i64().unwrap_or(0),
                    price_cents: row["priceCents"].as_i64().unwrap_or(0),
                    cost_cents: p.cost_cents,
                    tax_basis_points: row["taxBasisPoints"].as_i64().unwrap_or(0),
                    total_cents: row["totalCents"].as_i64().unwrap_or(0),
                    tax_cents: 0,
                });
            }
            let total = lines.iter().map(|x| x.total_cents).sum();
            let before = lines
                .iter()
                .map(|l| money::line_total(l.price_cents, l.quantity_units))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum::<i64>();
            let vat = tax(&mut lines)?;
            Charge {
                lines,
                total_cents: total,
                tax_cents: vat,
                discount_cents: before - total,
            }
        } else {
            calculate(&tx, &actor, &request.items, request.discount_parts)?
        };
        ensure(
            !charge.lines.is_empty(),
            "EMPTY_CART",
            "Agrega productos antes de cobrar.",
        )?;
        ensure(
            !request.payments.is_empty() && request.payments.len() <= 4,
            "VALIDATION",
            "Selecciona al menos un método de pago.",
        )?;
        let mut methods = HashSet::new();
        for p in &request.payments {
            ensure(
                (0..=3).contains(&p.method)
                    && (0..=money::MAX_AMOUNT).contains(&p.amount_cents)
                    && methods.insert(p.method)
                    && p.reference.len() <= 200,
                "VALIDATION",
                "Revisa los importes y no repitas un método de pago.",
            )?;
        }
        ensure(
            request.payments.iter().map(|p| p.amount_cents).sum::<i64>() == charge.total_cents,
            "PAYMENT_MISMATCH",
            "Los pagos deben sumar exactamente el total de la venta.",
        )?;
        let cash_amount = request
            .payments
            .iter()
            .filter(|p| p.method == 0)
            .map(|p| p.amount_cents)
            .sum::<i64>();
        ensure(
            request.tendered_cents >= cash_amount
                && request.tendered_cents <= money::MAX_AMOUNT
                && (cash_amount != 0 || request.tendered_cents == 0),
            "PAYMENT_MISMATCH",
            "El efectivo recibido debe cubrir el importe en efectivo.",
        )?;
        let customer = if let Some(id) = customer_id {
            let c = db::one(&tx, "SELECT * FROM Customer WHERE Id=?1 AND Active=1", [id])?;
            Some(c)
        } else {
            None
        };
        let credit = request
            .payments
            .iter()
            .filter(|p| p.method == 3)
            .map(|p| p.amount_cents)
            .sum::<i64>();
        if credit > 0 {
            ensure(
                customer.is_some(),
                "CUSTOMER_REQUIRED",
                "Selecciona un cliente para vender a crédito.",
            )?;
            let due = request.due_date.as_deref().unwrap_or("");
            ensure(
                chrono::NaiveDate::parse_from_str(due, "%Y-%m-%d")
                    .is_ok_and(|d| d.to_string() == due)
                    && due >= today.as_str(),
                "VALIDATION",
                "Indica una fecha de pago igual o posterior a hoy.",
            )?;
            let c = customer.as_ref().unwrap();
            let limit = c["creditLimitCents"].as_i64().unwrap_or(0);
            ensure(
                limit > 0 && credit_balance(&tx, customer_id.unwrap(), None)? + credit <= limit,
                "CREDIT_LIMIT",
                "La venta supera el límite de crédito del cliente. Solicita al administrador que lo revise.",
            )?;
        }
        let settings = db::one(&tx, "SELECT FolioPrefix FROM StoreSettings LIMIT 1", [])?;
        let folio = db::folio(
            &tx,
            "sales",
            settings["folioPrefix"].as_str().unwrap_or("V"),
        )?;
        let customer_name = customer
            .as_ref()
            .and_then(|c| c["name"].as_str())
            .unwrap_or("Público en general");
        tx.execute("INSERT INTO Sale(Folio,RequestId,CreatedAt,UserId,Cashier,CustomerId,CustomerName,CashSessionId,TotalCents,TaxCents,DiscountCents,ChangeCents,TenderedCents,Cancelled) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,0)",params![folio,request.request_id,now,actor.id,actor.name,customer_id,customer_name,cash,charge.total_cents,charge.tax_cents,charge.discount_cents,request.tendered_cents-cash_amount,request.tendered_cents])?;
        let sale_id = tx.last_insert_rowid();
        for l in &charge.lines {
            money::quantity(l.unit, l.quantity_units)?;
            catalog::stock(
                &tx,
                &now,
                actor.id,
                l.product_id,
                -l.quantity_units,
                "Venta",
                &folio,
                "",
            )?;
            tx.execute("INSERT INTO SaleLine(SaleId,ProductId,Sku,Name,Category,Unit,QuantityUnits,PriceCents,CostCents,TaxBasisPoints,TotalCents,TaxCents,ReturnedUnits,RefundedCents) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,0)",params![sale_id,l.product_id,l.sku,l.name,l.category,l.unit,l.quantity_units,l.price_cents,l.cost_cents,l.tax_basis_points,l.total_cents,l.tax_cents])?;
        }
        for p in &request.payments {
            if p.amount_cents > 0 {
                tx.execute("INSERT INTO PaymentRecord(SaleId,Method,AmountCents,Reference) VALUES (?1,?2,?3,?4)",params![sale_id,p.method,p.amount_cents,p.reference.trim()])?;
            }
        }
        cash_move(&tx, &now, actor.id, cash, cash_amount, "Venta", &folio, "")?;
        if credit > 0 {
            tx.execute("INSERT INTO CreditEntry(CustomerId,SaleId,CreatedAt,DueDate,AmountCents,Method,Kind,Reference,UserId) VALUES (?1,?2,?3,?4,?5,3,'Venta a crédito',?6,?7)",params![customer_id,sale_id,now,request.due_date,credit,folio,actor.id])?;
        }
        if let Some(id) = request.quote_id {
            tx.execute(
                "UPDATE Quote SET SaleId=?1 WHERE Id=?2",
                params![sale_id, id],
            )?;
        }
        if let Some(id) = request.held_sale_id {
            tx.execute(
                "UPDATE HeldSale SET SaleId=?1 WHERE Id=?2",
                params![sale_id, id],
            )?;
        }
        db::audit(&tx, &now, Some(actor.id), "Registrar venta", &folio, "")?;
        let detail = sale_detail(&tx, &actor, sale_id)?;
        operations::receipt(
            &tx,
            &actor,
            &request.request_id,
            "checkout",
            &hash,
            &json!({"id":sale_id}),
            &now,
        )?;
        tx.commit()?;
        Ok(detail)
    }
    pub fn sale(&self, id: i64) -> Result<Value> {
        let actor = self.actor("sell")?;
        sale_detail(self.db()?, &actor, id)
    }
    pub fn sales(&self, from: &str, to: &str, page: i64) -> Result<Value> {
        let actor = self.actor("sell")?;
        ensure(page >= 0, "VALIDATION", "La página no es válida.")?;
        let end = format!("{to} 23:59:59.9999999");
        let items = db::all(
            self.db()?,
            "SELECT * FROM Sale WHERE CreatedAt>=?1 AND CreatedAt<=?2 AND (?3=0 OR UserId=?4) ORDER BY Id DESC LIMIT 100 OFFSET ?5",
            params![from, end, actor.role, actor.id, page * 100],
        )?;
        Ok(json!(items))
    }
}
