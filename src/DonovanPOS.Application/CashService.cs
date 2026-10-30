using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class CashService(IDataStore store, UserSession session, IClock clock)
{
    public Task<CashOverview> Overview()
    {
        session.Require(Permission.Cash);
        return store.Read(async db =>
        {
            var open = (await db.List<CashSession>(x => x.ClosedAt == null)).SingleOrDefault();
            var rows = open is null ? [] : await db.List<CashMovement>(x => x.CashSessionId == open.Id, take: int.MaxValue);
            return new CashOverview(open, (open?.OpeningCents ?? 0) + rows.Sum(x => x.AmountCents), rows, await db.List<CashSession>(x => x.ClosedAt != null, take: 200));
        });
    }
    public Task<int> Open(decimal opening)
    {
        var actor = session.Require(Permission.Cash); Amount.Require(opening >= 0, "El fondo inicial no puede ser negativo.");
        return store.Write(async db =>
        {
            Amount.Require(await db.Count<CashSession>(x => x.ClosedAt == null) == 0, "Ya hay una caja abierta.");
            var cash = new CashSession { Folio = await Operations.Folio(db, "cash", "CJ"), UserId = actor.Id, OpenedAt = clock.Now, OpeningCents = Amount.Cents(opening) };
            db.Add(cash); await db.Flush(); Operations.Audit(db, clock, actor.Id, "Abrir caja", cash.Folio); return cash.Id;
        });
    }
    public Task<bool> Movement(decimal amount, string reason)
    {
        var actor = session.Require(Permission.Cash); reason = Amount.Text(reason, "Motivo", 300);
        Amount.Require(amount != 0, "Escribe un importe distinto de cero.");
        return store.Write(async db =>
        {
            var cash = await Operations.OpenCash(db);
            var balance = cash.OpeningCents + (await db.List<CashMovement>(x => x.CashSessionId == cash.Id, take: int.MaxValue)).Sum(x => x.AmountCents);
            Amount.Require(balance + Amount.Cents(amount) >= 0, "El egreso supera el efectivo esperado.");
            Operations.Cash(db, clock, actor.Id, cash.Id, Amount.Cents(amount), amount > 0 ? "Ingreso" : "Egreso", cash.Folio, reason);
            Operations.Audit(db, clock, actor.Id, "Movimiento de caja", cash.Folio, reason); return true;
        });
    }
    public Task<CashSession> Close(decimal counted, string notes)
    {
        var actor = session.Require(Permission.Cash); Amount.Require(counted >= 0, "El efectivo contado no puede ser negativo.");
        return store.Write(async db =>
        {
            var cash = await Operations.OpenCash(db);
            cash.ExpectedCents = cash.OpeningCents + (await db.List<CashMovement>(x => x.CashSessionId == cash.Id, take: int.MaxValue)).Sum(x => x.AmountCents);
            cash.CountedCents = Amount.Cents(counted); cash.ClosedAt = clock.Now; cash.Notes = notes.Trim();
            Operations.Audit(db, clock, actor.Id, "Cerrar caja", cash.Folio, $"Diferencia: {cash.CountedCents - cash.ExpectedCents} centavos"); return cash;
        });
    }
}
