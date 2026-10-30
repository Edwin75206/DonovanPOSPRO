using DonovanPOS.Domain;
namespace DonovanPOS.Application;

internal static class Operations
{
    public static async Task<string> Folio(IDataSession db, string key, string prefix)
    {
        var sequence = (await db.List<Sequence>(x => x.Key == key)).SingleOrDefault();
        if (sequence is null) { sequence = new() { Key = key }; db.Add(sequence); }
        sequence.Value++;
        return $"{prefix}-{sequence.Value:000000}";
    }
    public static void Audit(IDataSession db, IClock clock, int? userId, string action, string reference, string detail = "") =>
        db.Add(new AuditEvent { CreatedAt = clock.Now, UserId = userId, Action = action, Reference = reference, Detail = detail });
    public static void Stock(IDataSession db, IClock clock, int userId, Product product, long delta, string kind, string reference, string reason = "")
    {
        product.Move(delta);
        db.Add(new StockMovement { ProductId = product.Id, CreatedAt = clock.Now, UserId = userId, Kind = kind, Reference = reference, Reason = reason, DeltaUnits = delta, BalanceUnits = product.StockUnits });
    }
    public static async Task<CashSession> OpenCash(IDataSession db)
    {
        var cash = (await db.List<CashSession>(x => x.ClosedAt == null)).SingleOrDefault();
        Amount.Require(cash is not null, "Abre la caja antes de registrar esta operación.");
        return cash!;
    }
    public static void Cash(IDataSession db, IClock clock, int user, int session, long amount, string kind, string reference, string reason = "")
    {
        if (amount != 0) db.Add(new CashMovement { CashSessionId = session, UserId = user, CreatedAt = clock.Now, AmountCents = amount, Kind = kind, Reference = reference, Reason = reason });
    }
    public static async Task<long> Balance(IDataSession db, int customerId, int? saleId = null) =>
        (await db.List<CreditEntry>(x => x.CustomerId == customerId && (saleId == null || x.SaleId == saleId), take: int.MaxValue)).Sum(x => x.AmountCents);
}
