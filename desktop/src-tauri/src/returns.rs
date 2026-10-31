use crate::{
    Engine, catalog, commerce, db,
    error::{Result, ensure, text},
    money, operations,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnItem {
    pub sale_line_id: i64,
    pub quantity_units: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnInput {
    pub request_id: String,
    pub sale_id: i64,
    pub items: Vec<ReturnItem>,
    pub reason: String,
    pub method: i64,
    pub cancel: bool,
}
impl Engine {
    pub fn return_sale(&mut self, input: ReturnInput) -> Result<Value> {
        let actor = self.actor("return")?;
        let reason = text(&input.reason, "Motivo", 300)?;
        let hash = operations::fingerprint(&input)?;
        let now = self.now_string();
        ensure(
            (0..=2).contains(&input.method),
            "VALIDATION",
            "Selecciona cómo se entregó el reembolso.",
        )?;
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(result) = operations::previous(&tx, &actor, &input.request_id, "return", &hash)?
        {
            return Ok(result);
        }
        let sale = db::one(&tx, "SELECT * FROM Sale WHERE Id=?1", [input.sale_id])?;
        ensure(
            sale["cancelled"] == false,
            "ALREADY_COMPLETED",
            "La venta ya está cancelada o devuelta por completo.",
        )?;
        let lines = db::all(
            &tx,
            "SELECT * FROM SaleLine WHERE SaleId=?1 ORDER BY Id",
            [input.sale_id],
        )?;
        let requested = if input.cancel {
            lines
                .iter()
                .filter_map(|l| {
                    let units = l["quantityUnits"].as_i64()? - l["returnedUnits"].as_i64()?;
                    if units > 0 {
                        Some(ReturnItem {
                            sale_line_id: l["id"].as_i64()?,
                            quantity_units: units,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            input.items
        };
        ensure(
            !requested.is_empty() && requested.len() <= 500,
            "VALIDATION",
            "Selecciona las partidas que vas a devolver.",
        )?;
        let folio = db::folio(&tx, "returns", "DV")?;
        tx.execute("INSERT INTO SaleReturn(SaleId,CreatedAt,UserId,Folio,Reason,TotalCents,CreditReductionCents,RefundMethod) VALUES (?1,?2,?3,?4,?5,0,0,?6)",params![input.sale_id,now,actor.id,folio,reason,input.method])?;
        let id = tx.last_insert_rowid();
        let mut total = 0;
        let mut seen = HashSet::new();
        for item in requested {
            ensure(
                seen.insert(item.sale_line_id),
                "VALIDATION",
                "No repitas una partida en la devolución.",
            )?;
            let line = lines
                .iter()
                .find(|l| l["id"] == item.sale_line_id)
                .ok_or_else(|| {
                    crate::error::AppError::rule(
                        "VALIDATION",
                        "La partida no corresponde a esta venta.",
                    )
                })?;
            let units = line["quantityUnits"].as_i64().unwrap_or(0);
            let returned = line["returnedUnits"].as_i64().unwrap_or(0);
            money::quantity(line["unit"].as_i64().unwrap_or(-1), item.quantity_units)?;
            ensure(
                item.quantity_units <= units - returned,
                "VALIDATION",
                "La cantidad supera lo que falta devolver.",
            )?;
            let accumulated = money::rounded(
                line["totalCents"].as_i64().unwrap_or(0) as i128
                    * (returned + item.quantity_units) as i128,
                units as i128,
            );
            let refund = accumulated - line["refundedCents"].as_i64().unwrap_or(0);
            total += refund;
            tx.execute(
                "UPDATE SaleLine SET ReturnedUnits=?1,RefundedCents=?2 WHERE Id=?3",
                params![
                    returned + item.quantity_units,
                    accumulated,
                    item.sale_line_id
                ],
            )?;
            tx.execute("INSERT INTO ReturnLine(SaleReturnId,SaleLineId,QuantityUnits,AmountCents) VALUES (?1,?2,?3,?4)",params![id,item.sale_line_id,item.quantity_units,refund])?;
            catalog::stock(
                &tx,
                &now,
                actor.id,
                line["productId"].as_i64().unwrap_or(0),
                item.quantity_units,
                if input.cancel {
                    "Cancelación"
                } else {
                    "Devolución"
                },
                &folio,
                &reason,
            )?;
        }
        let mut credit_reduction = 0;
        if let Some(customer) = sale["customerId"].as_i64() {
            credit_reduction = commerce::credit_balance(&tx, customer, Some(input.sale_id))?
                .max(0)
                .min(total);
            if credit_reduction > 0 {
                tx.execute("INSERT INTO CreditEntry(CustomerId,SaleId,CreatedAt,DueDate,AmountCents,Method,Kind,Reference,UserId) VALUES (?1,?2,?3,NULL,?4,3,'Devolución a crédito',?5,?6)",params![customer,input.sale_id,now,-credit_reduction,folio,actor.id])?;
            }
        }
        let paid_refund = total - credit_reduction;
        if paid_refund > 0 && input.method == 0 {
            let cash = commerce::open_cash(&tx)?;
            commerce::cash_move(
                &tx,
                &now,
                actor.id,
                cash,
                -paid_refund,
                "Reembolso",
                &folio,
                &reason,
            )?;
        }
        tx.execute(
            "UPDATE SaleReturn SET TotalCents=?1,CreditReductionCents=?2 WHERE Id=?3",
            params![total, credit_reduction, id],
        )?;
        let cancelled = input.cancel
            || db::scalar(
                &tx,
                "SELECT COUNT(*) FROM SaleLine WHERE SaleId=?1 AND ReturnedUnits<QuantityUnits",
                [input.sale_id],
            )? == 0;
        tx.execute(
            "UPDATE Sale SET Cancelled=?1 WHERE Id=?2",
            params![cancelled, input.sale_id],
        )?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            if input.cancel {
                "Cancelar venta"
            } else {
                "Devolver productos"
            },
            &folio,
            &reason,
        )?;
        let result = json!({"return":db::one(&tx,"SELECT * FROM SaleReturn WHERE Id=?1",[id])?,"paidRefundCents":paid_refund,"cancelled":cancelled});
        operations::receipt(
            &tx,
            &actor,
            &input.request_id,
            "return",
            &hash,
            &result,
            &now,
        )?;
        tx.commit()?;
        Ok(result)
    }
}
