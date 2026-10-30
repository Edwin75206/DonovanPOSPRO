using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class ReturnService(IDataStore store, UserSession session, IClock clock)
{
    public Task<SaleReturn> Return(int saleId, IReadOnlyList<ReturnItem> items, string reason, PayMethod refundMethod, bool cancel = false)
    {
        var actor = session.Require(Permission.Return); reason = Amount.Text(reason, "Motivo", 300);
        Amount.Require(refundMethod is PayMethod.Efectivo or PayMethod.Tarjeta or PayMethod.Transferencia, "Selecciona cómo se entregó el reembolso.");
        return store.Write(async db =>
        {
            var sale = await db.Get<Sale>(saleId) ?? throw new RuleException("No se encontró la venta.");
            Amount.Require(!sale.Cancelled, "La venta ya está cancelada.");
            var lines = await db.List<SaleLine>(x => x.SaleId == saleId);
            var requested = cancel ? lines.Where(x => x.ReturnedUnits < x.QuantityUnits).Select(x => new ReturnItem(x.Id, Amount.Quantity(x.QuantityUnits - x.ReturnedUnits))).ToList() : items.ToList();
            Amount.Require(requested.Count > 0 && requested.Select(x => x.SaleLineId).Distinct().Count() == requested.Count, "Selecciona partidas distintas para devolver.");
            var result = new SaleReturn { SaleId = saleId, CreatedAt = clock.Now, UserId = actor.Id, Folio = await Operations.Folio(db, "returns", "DV"), Reason = reason, RefundMethod = refundMethod };
            db.Add(result); await db.Flush();
            foreach (var item in requested)
            {
                var line = lines.SingleOrDefault(x => x.Id == item.SaleLineId) ?? throw new RuleException("La partida no pertenece a esta venta.");
                var quantity = Amount.Units(item.Quantity); Amount.QuantityFor(line.Unit, quantity);
                Amount.Require(quantity <= line.QuantityUnits - line.ReturnedUnits, "La cantidad supera lo que falta devolver.");
                var newRefund = Amount.Cents(Amount.Pesos(line.TotalCents) * (line.ReturnedUnits + quantity) / line.QuantityUnits);
                var refund = newRefund - line.RefundedCents;
                line.ReturnedUnits += quantity; line.RefundedCents = newRefund; result.TotalCents += refund;
                db.Add(new ReturnLine { SaleReturnId = result.Id, SaleLineId = line.Id, QuantityUnits = quantity, AmountCents = refund });
                var p = await db.Get<Product>(line.ProductId) ?? throw new RuleException("No se encontró el producto.");
                Operations.Stock(db, clock, actor.Id, p, quantity, cancel ? "Cancelación" : "Devolución", result.Folio, reason);
            }
            if (sale.CustomerId is int customerId)
            {
                var debt = await Operations.Balance(db, customerId, saleId);
                result.CreditReductionCents = Math.Min(Math.Max(debt, 0), result.TotalCents);
                if (result.CreditReductionCents > 0) db.Add(new CreditEntry { CustomerId = customerId, SaleId = saleId, CreatedAt = clock.Now, UserId = actor.Id, AmountCents = -result.CreditReductionCents, Method = PayMethod.Credito, Kind = "Devolución a crédito", Reference = result.Folio });
            }
            var paidRefund = result.TotalCents - result.CreditReductionCents;
            if (paidRefund > 0 && refundMethod == PayMethod.Efectivo)
            {
                var cash = await Operations.OpenCash(db);
                var balance = cash.OpeningCents + (await db.List<CashMovement>(x => x.CashSessionId == cash.Id, take: int.MaxValue)).Sum(x => x.AmountCents);
                Amount.Require(balance >= paidRefund, "No hay suficiente efectivo en caja para este reembolso.");
                Operations.Cash(db, clock, actor.Id, cash.Id, -paidRefund, "Reembolso", result.Folio, reason);
            }
            sale.Cancelled = cancel || lines.All(x => x.ReturnedUnits == x.QuantityUnits);
            Operations.Audit(db, clock, actor.Id, cancel ? "Cancelar venta" : "Devolver productos", result.Folio, reason);
            return result;
        });
    }
}
