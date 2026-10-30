using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class CatalogService(IDataStore store, UserSession session, IClock clock, IProductQueries queries)
{
    public Task<ProductPage> Search(string search = "", int page = 0, bool archived = false, string? category = null, string? stockFilter = null, CancellationToken ct = default)
    {
        session.Require(Permission.Sell);
        return queries.Search(search, page, archived, category, stockFilter, ct);
    }
    public Task<List<string>> Categories() { session.Require(Permission.Sell); return queries.Categories(); }
    public Task<StockSummary> Summary() { session.Require(Permission.Stock); return queries.Summary(); }
    public Task<Product?> Get(int id) { session.Require(Permission.Sell); return store.Read(db => db.Get<Product>(id)); }
    public Task<int> Save(Product input)
    {
        var actor = session.Require(Permission.Catalog); input.Validate();
        return store.Write(async db =>
        {
            Amount.Require((await db.List<Product>(x => x.Id != input.Id && (x.Sku == input.Sku || (input.Barcode != null && x.Barcode == input.Barcode)))).Count == 0, "La clave o el código de barras ya está registrado.");
            var p = input.Id == 0 ? new Product() : await db.Get<Product>(input.Id) ?? throw new RuleException("No se encontró el producto.");
            if (input.Id != 0 && p.Unit != input.Unit)
                Amount.Require(await db.Count<StockMovement>(x => x.ProductId == p.Id) == 0, "La unidad base no puede cambiar después de registrar movimientos. Crea otro producto.");
            var previousPrice = p.PriceCents;
            p.Sku = input.Sku; p.Barcode = input.Barcode; p.Name = input.Name; p.Category = input.Category; p.Brand = input.Brand; p.Size = input.Size; p.Aliases = input.Aliases.Trim();
            p.Unit = input.Unit; p.CostCents = input.CostCents; p.PriceCents = input.PriceCents; p.WholesaleCents = input.WholesaleCents;
            p.MinimumUnits = input.MinimumUnits; p.PackName = input.PackName; p.PackUnits = input.PackUnits; p.TaxBasisPoints = input.TaxBasisPoints; p.Active = input.Active;
            if (input.Id == 0) { db.Add(p); await db.Flush(); if (input.StockUnits > 0) Operations.Stock(db, clock, actor.Id, p, input.StockUnits, "Existencia inicial", p.Sku); }
            Operations.Audit(db, clock, actor.Id, "Guardar producto", p.Sku, $"Precio anterior: {previousPrice} centavos; precio nuevo: {p.PriceCents} centavos");
            return p.Id;
        });
    }
    public Task<bool> Adjust(int productId, decimal delta, string reason)
    {
        var actor = session.Require(Permission.Stock); reason = Amount.Text(reason, "Motivo", 300);
        return store.Write(async db =>
        {
            var p = await db.Get<Product>(productId) ?? throw new RuleException("Selecciona un producto.");
            var units = Amount.Units(delta); Amount.QuantityFor(p.Unit, Math.Abs(units));
            Operations.Stock(db, clock, actor.Id, p, units, "Ajuste", p.Sku, reason);
            Operations.Audit(db, clock, actor.Id, "Ajuste de inventario", p.Sku, reason); return true;
        });
    }
    public Task<List<StockMovement>> Movements(int productId) { session.Require(Permission.Stock); return store.Read(db => db.List<StockMovement>(x => x.ProductId == productId, take: 1000)); }
}
