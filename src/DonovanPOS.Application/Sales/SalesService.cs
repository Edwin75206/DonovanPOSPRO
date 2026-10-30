using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class SalesService(IDataStore store, UserSession session, IClock clock)
{
    public Task<Sale> Checkout(CheckoutRequest request)
    {
        var actor = session.Require(Permission.Sell);
        Amount.Require(Guid.TryParse(request.RequestId, out _), "La referencia de venta no es válida. Inicia una venta nueva.");
        if (request.DiscountPercent != 0 || request.Items.Any(x => x.Wholesale)) session.Require(Permission.Discount);
        return store.Write(async db =>
        {
            var existing = (await db.List<Sale>(s => s.RequestId == request.RequestId)).SingleOrDefault();
            if (existing is not null) return existing;
            var cash = await Operations.OpenCash(db);
            HeldSale? held = null;
            if (request.HeldSaleId is int heldId)
            {
                held = await db.Get<HeldSale>(heldId) ?? throw new RuleException("No se encontró la venta suspendida.");
                Amount.Require(held.SaleId is null && !held.Discarded, "Esta venta suspendida ya se cerró.");
                Amount.Require(held.UserId == actor.Id || actor.Role == Role.Administrador, "Solo puedes cobrar tus ventas suspendidas.");
                Amount.Require(request.QuoteId is null, "Inicia por separado la venta de la cotización.");
            }
            Quote? quote = null;
            List<SaleLine> lines;
            if (request.QuoteId is int quoteId)
            {
                quote = await db.Get<Quote>(quoteId) ?? throw new RuleException("No se encontró la cotización.");
                Amount.Require(quote.SaleId is null, "Esta cotización ya se convirtió en venta.");
                Amount.Require(quote.ValidUntil.Date >= clock.Now.Date, "La cotización venció. Edita su vigencia antes de vender.");
                lines = [];
                foreach (var q in await db.List<QuoteLine>(x => x.QuoteId == quoteId))
                {
                    var p = await db.Get<Product>(q.ProductId) ?? throw new RuleException("Un producto ya no está disponible.");
                    Amount.Require(p.Active, $"{p.Name} está archivado.");
                    lines.Add(new SaleLine { ProductId = p.Id, Sku = p.Sku, Name = q.Name, Category = p.Category, Unit = q.Unit, QuantityUnits = q.QuantityUnits, PriceCents = q.PriceCents, CostCents = p.CostCents, TaxBasisPoints = q.TaxBasisPoints, TotalCents = q.TotalCents });
                }
            }
            else lines = await ChargeCalculator.Calculate(db, request.Items, request.DiscountPercent);
            Amount.Require(lines.Count > 0, "Agrega productos antes de cobrar.");
            var total = lines.Sum(x => x.TotalCents);
            Amount.Require(request.Payments.Count is > 0 and <= 4, "Selecciona al menos un método de pago.");
            Amount.Require(request.Payments.All(x => Enum.IsDefined(x.Method) && x.Amount >= 0 && x.Amount == Amount.Round(x.Amount)), "Revisa los importes de pago; usa dos decimales.");
            Amount.Require(request.Payments.Select(x => x.Method).Distinct().Count() == request.Payments.Count, "Cada método debe aparecer una sola vez.");
            Amount.Require(request.Payments.Sum(x => Amount.Cents(x.Amount)) == total, "Los pagos deben sumar exactamente el total de la venta.");
            var cashAmount = request.Payments.Where(x => x.Method == PayMethod.Efectivo).Sum(x => Amount.Cents(x.Amount));
            var tendered = Amount.Cents(request.Tendered);
            Amount.Require(tendered >= cashAmount && (cashAmount != 0 || tendered == 0), "El efectivo recibido debe cubrir el pago en efectivo.");
            var customerId = request.CustomerId ?? quote?.CustomerId;
            var customer = customerId is int id ? await db.Get<Customer>(id) : null;
            if (customerId is not null) Amount.Require(customer is { Active: true }, "Selecciona un cliente activo.");
            var credit = request.Payments.Where(x => x.Method == PayMethod.Credito).Sum(x => Amount.Cents(x.Amount));
            if (credit > 0)
            {
                Amount.Require(customer is not null, "Selecciona un cliente para la venta a crédito.");
                Amount.Require(request.DueDate is not null && request.DueDate.Value.Date >= clock.Now.Date, "Indica una fecha de pago igual o posterior a hoy.");
                var balance = await Operations.Balance(db, customer!.Id);
                Amount.Require(customer.CreditLimitCents > 0 && balance + credit <= customer.CreditLimitCents, "El crédito excede el límite del cliente. Solicita al administrador que lo revise.");
            }
            var settings = (await db.List<StoreSettings>()).Single();
            var sale = new Sale { RequestId = request.RequestId, Folio = await Operations.Folio(db, "sales", settings.FolioPrefix), CreatedAt = clock.Now, UserId = actor.Id, Cashier = actor.Name, CashSessionId = cash.Id, CustomerId = customer?.Id, CustomerName = customer?.Name ?? "Público en general", TotalCents = total, TaxCents = ChargeCalculator.Tax(lines), DiscountCents = lines.Sum(x => Amount.Cents(Amount.Pesos(x.PriceCents) * Amount.Quantity(x.QuantityUnits))) - total, TenderedCents = tendered, ChangeCents = tendered - cashAmount };
            db.Add(sale); await db.Flush();
            foreach (var line in lines)
            {
                var p = await db.Get<Product>(line.ProductId) ?? throw new RuleException("No se encontró el producto.");
                Operations.Stock(db, clock, actor.Id, p, -line.QuantityUnits, "Venta", sale.Folio);
                line.SaleId = sale.Id; db.Add(line);
            }
            foreach (var payment in request.Payments.Where(x => x.Amount > 0)) db.Add(new PaymentRecord { SaleId = sale.Id, Method = payment.Method, AmountCents = Amount.Cents(payment.Amount), Reference = payment.Reference.Trim() });
            Operations.Cash(db, clock, actor.Id, cash.Id, cashAmount, "Venta", sale.Folio);
            if (credit > 0) db.Add(new CreditEntry { CustomerId = customer!.Id, SaleId = sale.Id, UserId = actor.Id, CreatedAt = clock.Now, DueDate = request.DueDate!.Value.Date, AmountCents = credit, Method = PayMethod.Credito, Kind = "Venta a crédito", Reference = sale.Folio });
            if (quote is not null) quote.SaleId = sale.Id;
            if (held is not null) held.SaleId = sale.Id;
            Operations.Audit(db, clock, actor.Id, "Registrar venta", sale.Folio);
            return sale;
        });
    }
    public Task<List<Sale>> History(DateTime from, DateTime to, int page = 0)
    {
        var actor = session.Require(Permission.Sell); var end = to.Date.AddDays(1);
        return store.Read(db => db.List<Sale>(x => x.CreatedAt >= from.Date && x.CreatedAt < end && (actor.Role == Role.Administrador || x.UserId == actor.Id), page * 100, 100));
    }
    public Task<SaleDetail> Detail(int id)
    {
        var actor = session.Require(Permission.Sell);
        return store.Read(async db =>
        {
            var sale = await db.Get<Sale>(id) ?? throw new RuleException("No se encontró la venta.");
            Amount.Require(actor.Role == Role.Administrador || sale.UserId == actor.Id, "Solo puedes consultar tus ventas.");
            return new SaleDetail(sale, await db.List<SaleLine>(x => x.SaleId == id), await db.List<PaymentRecord>(x => x.SaleId == id), await db.List<SaleReturn>(x => x.SaleId == id));
        });
    }
}
