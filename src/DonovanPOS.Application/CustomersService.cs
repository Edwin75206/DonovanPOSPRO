using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class CustomersService(IDataStore store, UserSession session, IClock clock)
{
    public Task<List<Customer>> List(string search = "") { session.Require(Permission.Sell); return store.Read(db => db.List<Customer>(x => x.Active && x.Name.Contains(search), take: 1000)); }
    public Task<int> Save(Customer input)
    {
        var actor = session.Require(Permission.Credit); input.Name = Amount.Text(input.Name, "Nombre", 150);
        Amount.Require(input.CreditLimitCents >= 0, "El límite no puede ser negativo.");
        return store.Write(async db =>
        {
            var c = input.Id == 0 ? new Customer() : await db.Get<Customer>(input.Id) ?? throw new RuleException("No se encontró el cliente.");
            c.Name = input.Name; c.Phone = input.Phone.Trim(); c.Email = input.Email.Trim(); c.Address = input.Address.Trim(); c.CreditLimitCents = input.CreditLimitCents; c.Active = input.Active;
            if (input.Id == 0) db.Add(c); await db.Flush(); Operations.Audit(db, clock, actor.Id, "Guardar cliente", c.Id.ToString()); return c.Id;
        });
    }
    public Task<List<CreditEntry>> Ledger(int customerId) { session.Require(Permission.Credit); return store.Read(db => db.List<CreditEntry>(x => x.CustomerId == customerId, take: int.MaxValue)); }
    public Task<List<Sale>> Purchases(int customerId) { session.Require(Permission.Credit); return store.Read(db => db.List<Sale>(x => x.CustomerId == customerId, take: 1000)); }
    public Task<List<CustomerBalance>> Balances()
    {
        session.Require(Permission.Credit);
        return store.Read(async db => { var all = await db.List<CreditEntry>(take: int.MaxValue); return (await db.List<Customer>(take: int.MaxValue)).Select(c => new CustomerBalance(c, all.Where(x => x.CustomerId == c.Id).Sum(x => x.AmountCents))).ToList(); });
    }
    public Task<bool> Pay(int customerId, int saleId, decimal amount, PayMethod method, string reference)
    {
        var actor = session.Require(Permission.Credit); var cents = Amount.Cents(amount);
        Amount.Require(cents > 0 && method is PayMethod.Efectivo or PayMethod.Tarjeta or PayMethod.Transferencia, "Indica un abono mayor que cero y un método de pago válido.");
        return store.Write(async db =>
        {
            var sale = await db.Get<Sale>(saleId) ?? throw new RuleException("Selecciona una venta pendiente.");
            Amount.Require(sale.CustomerId == customerId, "La deuda no corresponde al cliente.");
            Amount.Require(cents <= await Operations.Balance(db, customerId, saleId), "El abono supera el saldo de esta venta.");
            var cash = await Operations.OpenCash(db);
            var folio = await Operations.Folio(db, "credit-payment", "AB");
            db.Add(new CreditEntry { CustomerId = customerId, SaleId = saleId, CreatedAt = clock.Now, UserId = actor.Id, AmountCents = -cents, Method = method, Kind = "Abono", Reference = folio + (string.IsNullOrWhiteSpace(reference) ? "" : " · " + reference.Trim()) });
            if (method == PayMethod.Efectivo) Operations.Cash(db, clock, actor.Id, cash.Id, cents, "Abono", folio);
            Operations.Audit(db, clock, actor.Id, "Registrar abono", folio, sale.Folio); return true;
        });
    }
}
