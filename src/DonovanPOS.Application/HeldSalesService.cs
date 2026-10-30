using DonovanPOS.Domain;
namespace DonovanPOS.Application;
public sealed record HeldSaleDetail(HeldSale Sale, List<HeldSaleLine> Lines, List<string> Notices);
public sealed class HeldSalesService(IDataStore store, UserSession session, IClock clock)
{
    public Task<List<HeldSale>> List()
    {
        var actor = session.Require(Permission.Sell);
        return store.Read(db => db.List<HeldSale>(x => x.SaleId == null && !x.Discarded && (actor.Role == Role.Administrador || x.UserId == actor.Id), take: 1000));
    }
    public Task<HeldSale> Save(int? existingId, string label, IReadOnlyList<SaleItem> items, int? customerId, decimal discount)
    {
        var actor = session.Require(Permission.Sell);
        if (discount != 0 || items.Any(x => x.Wholesale)) session.Require(Permission.Discount);
        Amount.Require(!string.IsNullOrWhiteSpace(label) && label.Length <= 100, "Asigna un nombre de hasta 100 caracteres a la venta suspendida.");
        return store.Write(async db =>
        {
            var priced = await ChargeCalculator.Calculate(db, items, discount);
            var sale = existingId is int id ? await db.Get<HeldSale>(id) ?? throw new RuleException("No se encontró la venta suspendida.") : new HeldSale { UserId = actor.Id, CreatedAt = clock.Now, Folio = await Operations.Folio(db, "held", "ESP") };
            Amount.Require(sale.SaleId is null && !sale.Discarded, "Esta venta suspendida ya se cerró.");
            Amount.Require(sale.UserId == actor.Id || actor.Role == Role.Administrador, "Solo puedes cambiar tus ventas suspendidas.");
            if (customerId is int cid) Amount.Require(await db.Get<Customer>(cid) is { Active: true }, "El cliente ya no está activo.");
            sale.Label = label.Trim(); sale.CustomerId = customerId; sale.DiscountParts = checked((int)(discount * 10000));
            if (sale.Id == 0) { db.Add(sale); await db.Flush(); }
            foreach (var old in await db.List<HeldSaleLine>(x => x.HeldSaleId == sale.Id)) db.Remove(old);
            foreach (var line in priced) db.Add(new HeldSaleLine { HeldSaleId = sale.Id, ProductId = line.ProductId, QuantityUnits = line.QuantityUnits, OriginalPriceCents = line.PriceCents, Wholesale = items.Single(x => x.ProductId == line.ProductId).Wholesale });
            Operations.Audit(db, clock, actor.Id, "Suspender venta", sale.Folio); return sale;
        });
    }
    public Task<HeldSaleDetail> Detail(int id)
    {
        var actor = session.Require(Permission.Sell);
        return store.Read(async db =>
        {
            var sale = await db.Get<HeldSale>(id) ?? throw new RuleException("No se encontró la venta suspendida.");
            Amount.Require(sale.SaleId is null && !sale.Discarded, "Esta venta suspendida ya se cerró.");
            Amount.Require(actor.Role == Role.Administrador || actor.Id == sale.UserId, "Solo puedes recuperar tus ventas suspendidas.");
            var lines = await db.List<HeldSaleLine>(x => x.HeldSaleId == id); List<string> notices = [];
            foreach (var line in lines)
            {
                var p = await db.Get<Product>(line.ProductId) ?? throw new RuleException("Falta un producto en el catálogo.");
                if (!p.Active) notices.Add(p.Name + ": producto archivado; retíralo antes de cobrar.");
                if (p.StockUnits < line.QuantityUnits) notices.Add(p.Name + ": existencias insuficientes; ajusta su cantidad.");
                if ((line.Wholesale ? p.WholesaleCents : p.PriceCents) != line.OriginalPriceCents) notices.Add(p.Name + ": se actualizó al precio vigente.");
            }
            if ((sale.DiscountParts != 0 || lines.Any(x => x.Wholesale)) && !session.Can(Permission.Discount)) notices.Add("Los descuentos y precios de mayoreo requieren administrador.");
            return new HeldSaleDetail(sale, lines, notices);
        });
    }
    public Task<int> Discard(int id)
    {
        var actor = session.Require(Permission.Sell); return store.Write(async db =>
        {
            var sale = await db.Get<HeldSale>(id) ?? throw new RuleException("No se encontró la venta.");
            Amount.Require(sale.UserId == actor.Id || actor.Role == Role.Administrador, "Solo puedes descartar tus ventas suspendidas.");
            Amount.Require(sale.SaleId is null, "Esta venta ya se cobró."); sale.Discarded = true; Operations.Audit(db, clock, actor.Id, "Descartar venta suspendida", sale.Folio); return sale.Id;
        });
    }
}
