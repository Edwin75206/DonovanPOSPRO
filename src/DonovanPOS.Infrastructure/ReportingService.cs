using System.Globalization;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Infrastructure.Persistence;
using Microsoft.EntityFrameworkCore;
namespace DonovanPOS.Infrastructure;

public sealed class ReportingService(Database database, UserSession session, IClock clock) : IReportingService
{
    public Task<DashboardData> Dashboard(DateTime from, DateTime to, CancellationToken ct = default)
    {
        session.Require(Permission.Reports); ValidateRange(from, to);
        return Task.Run(async () =>
        {
            await database.Gate.WaitAsync(ct);
            try
            {
                await using var db = database.Context(); var end = to.Date.AddDays(1); var start = from.Date; var today = clock.Now.Date;
                async Task<long> Net(DateTime a, DateTime z) => (await db.Sales.Where(x => x.CreatedAt >= a && x.CreatedAt < z).SumAsync(x => (long?)x.TotalCents, ct) ?? 0) - (await db.Set<SaleReturn>().Where(x => x.CreatedAt >= a && x.CreatedAt < z).SumAsync(x => (long?)x.TotalCents, ct) ?? 0);
                var dayCents = await Net(today, today.AddDays(1)); var weekCents = await Net(today.AddDays(-6), today.AddDays(1)); var monthCents = await Net(new(today.Year, today.Month, 1), today.AddDays(1));
                var total = await Net(start, end); var previous = await Net(start.AddDays(-(end - start).Days), start);
                var daily = await db.Sales.Where(x => x.CreatedAt >= start && x.CreatedAt < end).GroupBy(x => x.CreatedAt.Date).Select(g => new { Date = g.Key, Amount = g.Sum(x => x.TotalCents) }).ToListAsync(ct);
                var refunds = await db.Set<SaleReturn>().Where(x => x.CreatedAt >= start && x.CreatedAt < end).GroupBy(x => x.CreatedAt.Date).Select(g => new { Date = g.Key, Amount = g.Sum(x => x.TotalCents) }).ToListAsync(ct);
                var trend = Enumerable.Range(0, (end - start).Days).Select(i => start.AddDays(i)).Select(d => new ChartPoint(d.ToString("dd MMM", CultureInfo.GetCultureInfo("es-MX")), Amount.Pesos((daily.Find(x => x.Date == d)?.Amount ?? 0) - (refunds.Find(x => x.Date == d)?.Amount ?? 0)))).ToList();
                var saleIds = db.Sales.Where(x => x.CreatedAt >= start && x.CreatedAt < end).Select(x => x.Id);
                var periodLines = db.SaleLines.Where(x => saleIds.Contains(x.SaleId));
                var top = await periodLines.GroupBy(x => x.Name).Select(g => new { Name = g.Key, Amount = g.Sum(x => x.TotalCents) }).OrderByDescending(x => x.Amount).Take(5).ToListAsync(ct);
                var categories = await periodLines.GroupBy(x => x.Category).Select(g => new { Name = g.Key, Amount = g.Sum(x => x.TotalCents) }).OrderByDescending(x => x.Amount).Take(8).ToListAsync(ct);
                var methods = await db.Set<PaymentRecord>().Where(x => saleIds.Contains(x.SaleId)).GroupBy(x => x.Method).Select(g => new { Method = g.Key, Amount = g.Sum(x => x.AmountCents) }).ToListAsync(ct);
                var cash = await db.Set<CashSession>().SingleOrDefaultAsync(x => x.ClosedAt == null, ct);
                var cashCents = cash is null ? 0 : cash.OpeningCents + (await db.Set<CashMovement>().Where(x => x.CashSessionId == cash.Id).SumAsync(x => (long?)x.AmountCents, ct) ?? 0);
                var receivable = await db.Set<CreditEntry>().SumAsync(x => (long?)x.AmountCents, ct) ?? 0;
                var costLines = await periodLines.Select(x => new { x.TotalCents, x.TaxCents, x.CostCents, x.QuantityUnits }).ToListAsync(ct);
                var returnCosts = await (from r in db.Set<SaleReturn>() join rl in db.Set<ReturnLine>() on r.Id equals rl.SaleReturnId join sl in db.SaleLines on rl.SaleLineId equals sl.Id where r.CreatedAt >= start && r.CreatedAt < end select new { sl.CostCents, sl.TaxBasisPoints, rl.AmountCents, rl.QuantityUnits }).ToListAsync(ct);
                long? profit = costLines.Any(x => x.CostCents is null) || returnCosts.Any(x => x.CostCents is null) ? null : costLines.Sum(x => x.TotalCents - x.TaxCents - Amount.Cents(Amount.Pesos(x.CostCents!.Value) * Amount.Quantity(x.QuantityUnits))) - returnCosts.Sum(x => Amount.Cents(Amount.Pesos(x.AmountCents) / (1 + x.TaxBasisPoints / 10000m)) - Amount.Cents(Amount.Pesos(x.CostCents!.Value) * Amount.Quantity(x.QuantityUnits)));
                return new DashboardData(dayCents, await db.Sales.CountAsync(x => x.CreatedAt >= today && x.CreatedAt < today.AddDays(1), ct), weekCents, monthCents, previous, total, cashCents, receivable, profit, trend, top.Select(x => new ChartPoint(x.Name, Amount.Pesos(x.Amount))).ToList(), categories.Select(x => new ChartPoint(x.Name, Amount.Pesos(x.Amount))).ToList(), methods.Select(x => new ChartPoint(x.Method.ToString(), Amount.Pesos(x.Amount))).ToList(), await db.Products.Where(x => x.Active && x.StockUnits <= x.MinimumUnits).OrderBy(x => x.StockUnits).Take(8).ToListAsync(ct), await db.Sales.OrderByDescending(x => x.Id).Take(8).ToListAsync(ct));
            }
            finally { database.Gate.Release(); }
        }, ct);
    }
    public Task<TableData> Report(string kind, DateTime from, DateTime to, CancellationToken ct = default)
    {
        session.Require(Permission.Reports); ValidateRange(from, to);
        return Task.Run(async () =>
        {
            await database.Gate.WaitAsync(ct);
            try
            {
                await using var db = database.Context(); var end = to.Date.AddDays(1); var start = from.Date;
                var saleIds = db.Sales.Where(x => x.CreatedAt >= start && x.CreatedAt < end).Select(x => x.Id);
                string Money(long value) => Amount.Pesos(value).ToString("C2", CultureInfo.GetCultureInfo("es-MX"));
                var title = kind + " · " + start.ToString("dd/MM/yyyy") + " — " + to.ToString("dd/MM/yyyy");
                if (kind == "Inventario")
                {
                    var rows = await db.Products.AsNoTracking().OrderBy(x => x.Name).ToListAsync(ct);
                    return new TableData(title, ["Clave", "Producto", "Unidad", "Existencia", "Costo neto", "Precio MXN", "Estado"], rows.Select(x => new[] { x.Sku, x.Name, x.Unit.ToString(), x.Stock.ToString("0.####"), x.CostCents is long c ? Money(c) : "Sin costo", Money(x.PriceCents), x.StockStatus }).ToList());
                }
                if (kind == "Cuentas por cobrar")
                {
                    var balances = await (from c in db.Set<Customer>() join e in db.Set<CreditEntry>() on c.Id equals e.CustomerId group e by new { c.Name, c.Phone } into g select new { g.Key.Name, g.Key.Phone, Balance = g.Sum(x => x.AmountCents) }).ToListAsync(ct);
                    return new TableData("Cuentas por cobrar · saldo actual", ["Cliente", "Teléfono", "Saldo MXN"], balances.Where(x => x.Balance != 0).Select(x => new[] { x.Name, x.Phone, Money(x.Balance) }).ToList());
                }
                if (kind == "Compras")
                {
                    var rows = await (from p in db.Set<Purchase>() join s in db.Set<Supplier>() on p.SupplierId equals s.Id where p.CreatedAt >= start && p.CreatedAt < end orderby p.Id descending select new { p.Folio, p.SupplierReference, p.CreatedAt, s.Name, p.TotalCents }).ToListAsync(ct);
                    return new TableData(title, ["Folio", "Proveedor", "Referencia", "Fecha", "Costo neto MXN"], rows.Select(x => new[] { x.Folio, x.Name, x.SupplierReference, x.CreatedAt.ToString("dd/MM/yyyy"), Money(x.TotalCents) }).ToList());
                }
                if (kind == "Devoluciones")
                {
                    var rows = await db.Set<SaleReturn>().Where(x => x.CreatedAt >= start && x.CreatedAt < end).OrderByDescending(x => x.Id).ToListAsync(ct);
                    return new TableData(title, ["Folio", "Fecha", "Total MXN", "Crédito reducido", "Reembolso", "Motivo"], rows.Select(x => new[] { x.Folio, x.CreatedAt.ToString("dd/MM/yyyy"), Money(x.TotalCents), Money(x.CreditReductionCents), x.RefundMethod.ToString(), x.Reason }).ToList());
                }
                if (kind == "Métodos de pago")
                {
                    var rows = await db.Set<PaymentRecord>().Where(x => saleIds.Contains(x.SaleId)).GroupBy(x => x.Method).Select(g => new { Method = g.Key, Total = g.Sum(x => x.AmountCents) }).ToListAsync(ct);
                    return new TableData(title + " · pagos de ventas antes de devoluciones", ["Método", "Importe MXN"], rows.Select(x => new[] { x.Method.ToString(), Money(x.Total) }).ToList());
                }
                if (kind is "Por producto" or "Por categoría")
                {
                    var rows = await db.SaleLines.Where(x => saleIds.Contains(x.SaleId)).GroupBy(x => kind == "Por producto" ? x.Name : x.Category).Select(g => new { Name = g.Key, Gross = g.Sum(x => x.TotalCents), Refund = g.Sum(x => x.RefundedCents) }).OrderByDescending(x => x.Gross).ToListAsync(ct);
                    return new TableData(title, ["Concepto", "Venta original MXN", "Devuelto acumulado", "Venta restante"], rows.Select(x => new[] { x.Name, Money(x.Gross), Money(x.Refund), Money(x.Gross - x.Refund) }).ToList());
                }
                if (kind == "Por usuario")
                {
                    var rows = await db.Sales.Where(x => saleIds.Contains(x.Id)).GroupBy(x => x.Cashier).Select(g => new { Name = g.Key, Count = g.Count(), Total = g.Sum(x => x.TotalCents) }).ToListAsync(ct);
                    return new TableData(title, ["Usuario", "Operaciones", "Venta original MXN"], rows.Select(x => new[] { x.Name, x.Count.ToString(), Money(x.Total) }).ToList());
                }
                if (kind == "Costos y ganancias")
                {
                    var rows = await db.SaleLines.Where(x => saleIds.Contains(x.SaleId)).AsNoTracking().ToListAsync(ct);
                    return new TableData(title + " · estimación de ventas restantes", ["Producto", "Cantidad restante", "Venta sin IVA", "Costo estimado", "Ganancia estimada"], rows.Select(x => { var net = Amount.Cents(Amount.Pesos(x.TotalCents - x.RefundedCents) / (1 + x.TaxBasisPoints / 10000m)); long? cost = x.CostCents is null ? null : Amount.Cents(Amount.Pesos(x.CostCents.Value) * Amount.Quantity(x.QuantityUnits - x.ReturnedUnits)); return new[] { x.Name, Amount.Quantity(x.QuantityUnits - x.ReturnedUnits).ToString("0.####"), Money(net), cost.HasValue ? Money(cost.Value) : "Costo faltante", cost.HasValue ? Money(net - cost.Value) : "No calculable" }; }).ToList());
                }
                var sales = await db.Sales.Where(x => x.CreatedAt >= start && x.CreatedAt < end).OrderByDescending(x => x.Id).AsNoTracking().ToListAsync(ct);
                return new TableData(title, ["Folio", "Fecha", "Cliente", "Cajero", "Total original", "Estado"], sales.Select(x => new[] { x.Folio, x.CreatedAt.ToString("dd/MM/yyyy HH:mm"), x.CustomerName, x.Cashier, Money(x.TotalCents), x.Cancelled ? "Cancelada / devuelta" : "Registrada" }).ToList());
            }
            finally { database.Gate.Release(); }
        }, ct);
    }
    private static void ValidateRange(DateTime from, DateTime to) => Amount.Require(to.Date >= from.Date && (to.Date - from.Date).Days <= 366, "Selecciona un rango de hasta 367 días, con la fecha final después de la inicial.");
}

