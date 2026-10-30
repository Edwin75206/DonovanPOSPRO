using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class PurchasesService(IDataStore store, UserSession session, IClock clock)
{
    public Task<List<Supplier>> Suppliers() { session.Require(Permission.Purchase); return store.Read(db => db.List<Supplier>(x => x.Active)); }
    public Task<List<Purchase>> History() { session.Require(Permission.Purchase); return store.Read(db => db.List<Purchase>(take: 500)); }
    public Task<int> SaveSupplier(Supplier input)
    {
        var actor = session.Require(Permission.Purchase);
        return store.Write(async db =>
        {
            var s = input.Id == 0 ? new Supplier() : await db.Get<Supplier>(input.Id) ?? throw new RuleException("No se encontró el proveedor.");
            s.Name = Amount.Text(input.Name, "Proveedor", 150); s.Phone = input.Phone.Trim(); s.Email = input.Email.Trim(); s.Address = input.Address.Trim();
            if (input.Id == 0) db.Add(s); await db.Flush(); Operations.Audit(db, clock, actor.Id, "Guardar proveedor", s.Id.ToString()); return s.Id;
        });
    }
    public Task<Purchase> Receive(int supplierId, string reference, DateTime date, IReadOnlyList<PurchaseItem> items, bool paidFromCash)
    {
        var actor = session.Require(Permission.Purchase); reference = Amount.Text(reference, "Folio del proveedor", 100);
        Amount.Require(items.Count is > 0 and <= 500, "Agrega productos a la compra.");
        return store.Write(async db =>
        {
            Amount.Require(await db.Get<Supplier>(supplierId) is { Active: true }, "Selecciona un proveedor activo.");
            Amount.Require(await db.Count<Purchase>(x => x.SupplierId == supplierId && x.SupplierReference == reference) == 0, "Este folio del proveedor ya fue recibido.");
            var purchase = new Purchase { SupplierId = supplierId, SupplierReference = reference, Folio = await Operations.Folio(db, "purchases", "CP"), CreatedAt = date, UserId = actor.Id };
            db.Add(purchase); await db.Flush();
            foreach (var item in items)
            {
                var p = await db.Get<Product>(item.ProductId) ?? throw new RuleException("No se encontró un producto.");
                Amount.Require(item.Quantity > 0 && item.UnitCost >= 0, "Revisa la cantidad y el costo de compra.");
                if (item.ByPack) Amount.Require(item.Quantity == decimal.Truncate(item.Quantity), "Las presentaciones de compra se reciben completas.");
                var quantity = item.ByPack ? Amount.Units(item.Quantity * Amount.Quantity(p.PackUnits)) : Amount.Units(item.Quantity);
                Amount.QuantityFor(p.Unit, quantity);
                var unitCost = item.ByPack ? Amount.Cents(item.UnitCost / Amount.Quantity(p.PackUnits)) : Amount.Cents(item.UnitCost);
                var lineTotal = Amount.Cents(item.UnitCost * item.Quantity);
                purchase.TotalCents += lineTotal;
                // Weighted average, in base units. Last purchase is used when stock has no known cost.
                p.CostCents = p.CostCents is null || p.StockUnits == 0 ? unitCost : Amount.Cents((Amount.Pesos(p.CostCents.Value) * p.StockUnits + Amount.Pesos(unitCost) * quantity) / (p.StockUnits + quantity));
                Operations.Stock(db, clock, actor.Id, p, quantity, "Compra", purchase.Folio);
                db.Add(new PurchaseLine { PurchaseId = purchase.Id, ProductId = p.Id, QuantityUnits = quantity, CostCents = unitCost });
            }
            if (paidFromCash)
            {
                var cash = await Operations.OpenCash(db);
                var expected = cash.OpeningCents + (await db.List<CashMovement>(x => x.CashSessionId == cash.Id, take: int.MaxValue)).Sum(x => x.AmountCents);
                Amount.Require(expected >= purchase.TotalCents, "No hay efectivo suficiente en caja para pagar la compra.");
                Operations.Cash(db, clock, actor.Id, cash.Id, -purchase.TotalCents, "Compra", purchase.Folio);
            }
            Operations.Audit(db, clock, actor.Id, "Recibir compra", purchase.Folio); return purchase;
        });
    }
}
