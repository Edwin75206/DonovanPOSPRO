using DonovanPOS.Domain;
using DonovanPOS.Domain.Pricing;
namespace DonovanPOS.Application;

internal static class ChargeCalculator
{
    public static async Task<List<SaleLine>> Calculate(IDataSession db, IReadOnlyList<SaleItem> items, decimal discountPercent)
    {
        Amount.Require(items.Count is > 0 and <= 500, "Agrega entre 1 y 500 partidas.");
        Amount.Require(items.Select(x => x.ProductId).Distinct().Count() == items.Count, "Agrupa las cantidades del mismo producto en una sola partida.");
        Amount.Require(discountPercent is >= 0 and <= 100, "El descuento debe estar entre 0 y 100%.");
        var cart = new Cart();
        foreach (var item in items)
        {
            var p = await db.Get<Product>(item.ProductId) ?? throw new RuleException("No se encontró un producto de la venta.");
            Amount.Require(p.Active, $"{p.Name} está archivado.");
            Amount.QuantityFor(p.Unit, Amount.Units(item.Quantity));
            // A detached pricing snapshot keeps wholesale selection from changing the catalog.
            var pricing = new Product { Id = p.Id, Sku = p.Sku, Name = p.Name, Category = p.Category, Unit = p.Unit, PriceCents = item.Wholesale ? p.WholesaleCents : p.PriceCents, CostCents = p.CostCents, TaxBasisPoints = p.TaxBasisPoints };
            cart.AddOrIncrement(pricing, item.Quantity);
        }
        if (discountPercent != 0) cart.ApplyCartDiscount(new PercentageDiscount(discountPercent / 100m));
        return PricingService.Summarize(cart).LineCharges.Select(c => new SaleLine
        {
            ProductId = c.Line.Product.Id, Sku = c.Line.Product.Sku, Name = c.Line.Product.Name, Category = c.Line.Product.Category, Unit = c.Line.Product.Unit,
            QuantityUnits = Amount.Units(c.Line.Quantity), PriceCents = c.Line.Product.PriceCents, CostCents = c.Line.Product.CostCents,
            TaxBasisPoints = c.Line.Product.TaxBasisPoints, TotalCents = Amount.Cents(c.FinalGross)
        }).ToList();
    }
    public static long Tax(List<SaleLine> lines)
    {
        foreach (var group in lines.GroupBy(x => x.TaxBasisPoints))
        {
            var rate = group.Key / 10000m;
            var gross = group.Sum(x => x.TotalCents);
            var tax = gross - Amount.Cents(Amount.Pesos(gross) / (1 + rate));
            var rows = group.ToArray(); var allocations = Amount.Allocate(tax, rows.Select(x => x.TotalCents).ToArray());
            for (var i = 0; i < rows.Length; i++) rows[i].TaxCents = allocations[i];
        }
        return lines.Sum(x => x.TaxCents);
    }
}
