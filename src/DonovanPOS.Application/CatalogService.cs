using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class CatalogService(IDataStore store, UserSession session, IClock clock)
{
    public Task<ProductPage> Search(string search = "", int page = 0, bool archived = false)
    {
        session.Require(Permission.Sell);
        var term = search.Trim().ToLowerInvariant();
        return store.Read(async db => new ProductPage(await db.List<Product>(p => (archived || p.Active) && (p.Name.ToLower().Contains(term) || p.Sku.ToLower().Contains(term) || (p.Barcode != null && p.Barcode.Contains(term)) || p.Category.ToLower().Contains(term) || p.Brand.ToLower().Contains(term)), Math.Max(0, page) * 100, 100), await db.Count<Product>(p => (archived || p.Active) && (p.Name.ToLower().Contains(term) || p.Sku.ToLower().Contains(term) || (p.Barcode != null && p.Barcode.Contains(term)) || p.Category.ToLower().Contains(term) || p.Brand.ToLower().Contains(term)))));
    }
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
            p.Sku = input.Sku; p.Barcode = input.Barcode; p.Name = input.Name; p.Category = input.Category; p.Brand = input.Brand; p.Size = input.Size;
            p.Unit = input.Unit; p.CostCents = input.CostCents; p.PriceCents = input.PriceCents; p.WholesaleCents = input.WholesaleCents;
            p.MinimumUnits = input.MinimumUnits; p.PackName = input.PackName; p.PackUnits = input.PackUnits; p.TaxBasisPoints = input.TaxBasisPoints; p.Active = input.Active;
            if (input.Id == 0) { db.Add(p); await db.Flush(); if (input.StockUnits > 0) Operations.Stock(db, clock, actor.Id, p, input.StockUnits, "Existencia inicial", p.Sku); }
            Operations.Audit(db, clock, actor.Id, "Guardar producto", p.Sku);
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
