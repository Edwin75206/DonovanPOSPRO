using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class QuotesService(IDataStore store, UserSession session, IClock clock)
{
    public Task<List<Quote>> List() { session.Require(Permission.Quote); return store.Read(db => db.List<Quote>(take: 500)); }
    public Task<QuoteDetail> Detail(int id) { session.Require(Permission.Quote); return store.Read(async db => new QuoteDetail(await db.Get<Quote>(id) ?? throw new RuleException("No se encontró la cotización."), await db.List<QuoteLine>(x => x.QuoteId == id))); }
    public Task<Quote> Save(int id, IReadOnlyList<SaleItem> items, int? customerId, decimal discount, DateTime validUntil, string notes)
    {
        var actor = session.Require(Permission.Quote); if (discount != 0 || items.Any(x => x.Wholesale)) session.Require(Permission.Discount);
        Amount.Require(validUntil.Date >= clock.Now.Date, "La vigencia debe ser igual o posterior a hoy.");
        return store.Write(async db =>
        {
            var quote = id == 0 ? new Quote { CreatedAt = clock.Now, UserId = actor.Id, Folio = await Operations.Folio(db, "quotes", "CT") } : await db.Get<Quote>(id) ?? throw new RuleException("No se encontró la cotización.");
            Amount.Require(quote.SaleId is null, "Una cotización vendida conserva su historial y no puede editarse.");
            var lines = await ChargeCalculator.Calculate(db, items, discount);
            quote.CustomerId = customerId; quote.CustomerName = customerId is int cid ? (await db.Get<Customer>(cid) ?? throw new RuleException("No se encontró el cliente.")).Name : "Público en general";
            quote.ValidUntil = validUntil.Date; quote.Notes = notes.Trim(); quote.TotalCents = lines.Sum(x => x.TotalCents); quote.TaxCents = ChargeCalculator.Tax(lines);
            if (id == 0) db.Add(quote); await db.Flush();
            foreach (var old in await db.List<QuoteLine>(x => x.QuoteId == quote.Id)) db.Remove(old);
            foreach (var l in lines) db.Add(new QuoteLine { QuoteId = quote.Id, ProductId = l.ProductId, Name = l.Name, Unit = l.Unit, QuantityUnits = l.QuantityUnits, PriceCents = l.PriceCents, TaxBasisPoints = l.TaxBasisPoints, TotalCents = l.TotalCents });
            Operations.Audit(db, clock, actor.Id, id == 0 ? "Crear cotización" : "Editar cotización", quote.Folio); return quote;
        });
    }
}
