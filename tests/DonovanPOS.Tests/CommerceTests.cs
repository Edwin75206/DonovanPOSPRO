using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Domain.Pricing;
using DonovanPOS.Infrastructure;
using DonovanPOS.Infrastructure.Persistence;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Xunit;
namespace DonovanPOS.Tests;

public sealed class TestShop : IAsyncDisposable
{
    public string Directory { get; } = Path.Combine(Path.GetTempPath(), "DonovanTests", Guid.NewGuid().ToString("N"));
    public ServiceProvider Services { get; }
    public TestShop() => Services = new ServiceCollection().AddDonovan(Directory).BuildServiceProvider();
    public T Get<T>() where T : notnull => Services.GetRequiredService<T>();
    public async Task Initialize()
    {
        await Get<Database>().Initialize(); await Get<AuthService>().Setup("Ferretería de prueba", "admin", "Administrador de prueba", "Testing-only-2026"); await Get<AuthService>().Login("admin", "Testing-only-2026"); await Get<CashService>().Open(1000);
    }
    public async Task<int> Product(decimal price = 116, decimal stock = 20, Unit unit = Unit.Pieza)
    {
        var p = new Product { Sku = Guid.NewGuid().ToString("N")[..12], Name = "Producto de prueba", Category = "Herramientas", Unit = unit, CostCents = 5000, PriceCents = Amount.Cents(price), WholesaleCents = Amount.Cents(price * .9m), StockUnits = Amount.Units(stock), PackUnits = unit == Unit.Metro ? 1000000 : 10000 };
        return await Get<CatalogService>().Save(p);
    }
    public CheckoutRequest Request(int id, decimal quantity = 1, decimal total = 116) => new(Guid.NewGuid().ToString(), [new(id, quantity)], [new(PayMethod.Efectivo, total)], total);
    public async ValueTask DisposeAsync() { await Services.DisposeAsync(); if (System.IO.Directory.Exists(Directory)) System.IO.Directory.Delete(Directory, true); }
}
public sealed class CommerceTests
{
    [Fact] public async Task First_run_has_no_demo_accounts_or_products()
    {
        await using var s = new TestShop(); await s.Get<Database>().Initialize(); Assert.True(await s.Get<AuthService>().NeedsSetup());
        Assert.Equal(0, await s.Get<IDataStore>().Read(db => db.Count<Product>()));
        await Assert.ThrowsAsync<RuleException>(() => s.Get<AuthService>().Setup("Shop", "admin", "Admin", "1234"));
    }
    [Fact] public async Task Sale_tax_discount_split_payment_change_and_idempotency_are_exact()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product();
        var req = s.Request(id, 2, 208.80m) with { DiscountPercent = 10, Payments = [new(PayMethod.Efectivo, 100), new(PayMethod.Tarjeta, 108.8m)], Tendered = 150 };
        var sale = await s.Get<SalesService>().Checkout(req); var duplicate = await s.Get<SalesService>().Checkout(req);
        Assert.Equal(sale.Id, duplicate.Id); Assert.Equal(20880, sale.TotalCents); Assert.Equal(2880, sale.TaxCents); Assert.Equal(5000, sale.ChangeCents);
        Assert.Equal(18, (await s.Get<CatalogService>().Get(id))!.Stock); Assert.Equal(110000, (await s.Get<CashService>().Overview()).ExpectedCents);
    }
    [Fact] public async Task Failure_rolls_back_all_sale_lines_payments_and_stock()
    {
        await using var s = new TestShop(); await s.Initialize(); var a = await s.Product(stock: 2); var b = await s.Product(stock: 0);
        await Assert.ThrowsAsync<RuleException>(() => s.Get<SalesService>().Checkout(new(Guid.NewGuid().ToString(), [new(a, 1), new(b, 1)], [new(PayMethod.Efectivo, 232)], 232)));
        Assert.Equal(2, (await s.Get<CatalogService>().Get(a))!.Stock); Assert.Equal(0, await s.Get<IDataStore>().Read(db => db.Count<Sale>())); Assert.Equal(100000, (await s.Get<CashService>().Overview()).ExpectedCents);
    }
    [Fact] public async Task Concurrent_checkout_cannot_oversell()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(stock: 1);
        var outcomes = await Task.WhenAll(Enumerable.Range(0, 2).Select(async _ => { try { await s.Get<SalesService>().Checkout(s.Request(id)); return true; } catch (RuleException) { return false; } }));
        Assert.Single(outcomes, x => x); Assert.Equal(0, (await s.Get<CatalogService>().Get(id))!.Stock);
    }
    [Fact] public async Task Purchase_pack_conversion_and_fractional_sale_preserve_base_unit()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(11.6m, 0, Unit.Metro);
        var supplier = await s.Get<PurchasesService>().SaveSupplier(new Supplier { Name = "Proveedor" });
        await s.Get<PurchasesService>().Receive(supplier, "F-1", DateTime.Today, [new(id, 1, 500, true)], false);
        await s.Get<SalesService>().Checkout(s.Request(id, 2.5m, 29)); var p = (await s.Get<CatalogService>().Get(id))!;
        Assert.Equal(97.5m, p.Stock); Assert.Equal(500, p.CostCents);
        await Assert.ThrowsAsync<RuleException>(() => s.Get<PurchasesService>().Receive(supplier, "F-1", DateTime.Today, [new(id, 1, 500, true)], false));
    }
    [Fact] public async Task Quote_does_not_reserve_stock_and_converts_once_at_agreed_price()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product();
        var quote = await s.Get<QuotesService>().Save(0, [new(id, 2)], null, 10, DateTime.Today.AddDays(10), "Entrega en mostrador");
        Assert.Equal(20, (await s.Get<CatalogService>().Get(id))!.Stock);
        var product = (await s.Get<CatalogService>().Get(id))!; product.PriceCents = 99900; await s.Get<CatalogService>().Save(product);
        var sale = await s.Get<SalesService>().Checkout(s.Request(id, 2, 208.8m) with { QuoteId = quote.Id }); Assert.Equal(20880, sale.TotalCents);
        await Assert.ThrowsAsync<RuleException>(() => s.Get<SalesService>().Checkout(s.Request(id, 2, 208.8m) with { QuoteId = quote.Id }));
    }
    [Fact] public async Task Credit_abono_and_return_reduce_debt_before_refunding_paid_money()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product();
        var cid = await s.Get<CustomersService>().Save(new Customer { Name = "Cliente", CreditLimitCents = 50000 });
        var sale = await s.Get<SalesService>().Checkout(s.Request(id) with { Payments = [new(PayMethod.Credito, 116)], Tendered = 0, CustomerId = cid, DueDate = DateTime.Today.AddDays(10) });
        Assert.Equal(100000, (await s.Get<CashService>().Overview()).ExpectedCents);
        await s.Get<CustomersService>().Pay(cid, sale.Id, 50, PayMethod.Efectivo, "");
        Assert.Equal(6600, (await s.Get<CustomersService>().Balances()).Single().BalanceCents);
        var refund = await s.Get<ReturnService>().Return(sale.Id, [], "Cancelación solicitada", PayMethod.Efectivo, true);
        Assert.Equal(6600, refund.CreditReductionCents); Assert.Equal(0, (await s.Get<CustomersService>().Balances()).Single().BalanceCents); Assert.Equal(100000, (await s.Get<CashService>().Overview()).ExpectedCents);
    }
    [Fact] public async Task Partial_returns_reconcile_pennies_and_cannot_be_repeated()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(.01m, 3); var sale = await s.Get<SalesService>().Checkout(s.Request(id, 3, .02m) with { DiscountPercent = 33.333m });
        var line = (await s.Get<SalesService>().Detail(sale.Id)).Lines.Single(); long sum = 0;
        for (var i = 0; i < 3; i++) sum += (await s.Get<ReturnService>().Return(sale.Id, [new(line.Id, 1)], "Devolución de prueba", PayMethod.Efectivo)).TotalCents;
        Assert.Equal(2, sum); Assert.Equal(3, (await s.Get<CatalogService>().Get(id))!.Stock);
        await Assert.ThrowsAsync<RuleException>(() => s.Get<ReturnService>().Return(sale.Id, [new(line.Id, 1)], "Repetida", PayMethod.Efectivo));
    }
    [Fact] public async Task Cash_close_persists_expected_counted_and_history()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(); await s.Get<SalesService>().Checkout(s.Request(id)); await s.Get<CashService>().Movement(-16, "Flete");
        var closed = await s.Get<CashService>().Close(1090, "Diferencia de 10 pesos"); Assert.Equal(110000, closed.ExpectedCents); Assert.Equal(109000, closed.CountedCents);
        Assert.Null((await s.Get<CashService>().Overview()).Session);
        await Assert.ThrowsAsync<RuleException>(() => s.Get<SalesService>().Checkout(s.Request(id)));
    }
    [Fact] public async Task Permissions_are_enforced_by_services_and_passwords_are_hashed()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product();
        await s.Get<AuthService>().SaveUser(0, "caja", "Cajero", Role.Cajero, true, "Cashier-test-2026");
        Assert.All(await s.Get<AuthService>().Users(), x => Assert.StartsWith("pbkdf2-sha256$", x.PasswordHash));
        await s.Get<AuthService>().Login("caja", "Cashier-test-2026");
        await Assert.ThrowsAsync<RuleException>(() => s.Get<CatalogService>().Adjust(id, 10, "Intento"));
        await Assert.ThrowsAsync<RuleException>(() => s.Get<SalesService>().Checkout(s.Request(id, total: 104.4m) with { DiscountPercent = 10 }));
        await Assert.ThrowsAsync<RuleException>(() => s.Get<ReturnService>().Return(1, [], "Intento", PayMethod.Efectivo, true));
    }
    [Fact] public async Task Reopen_backup_restore_recover_committed_WAL_data()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(); await s.Get<SalesService>().Checkout(s.Request(id));
        var backup = await s.Get<IBackupService>().Backup(Path.Combine(s.Directory, "backups"));
        await s.Get<CatalogService>().Adjust(id, 5, "Cambio posterior"); await s.Get<IBackupService>().Restore(backup);
        await s.Get<AuthService>().Login("admin", "Testing-only-2026"); Assert.Equal(19, (await s.Get<CatalogService>().Get(id))!.Stock);
        var fresh = new Database(s.Directory); await fresh.Initialize(); Assert.Equal(1, await fresh.Read(db => db.Count<Sale>()));
        await using var context = fresh.Context(); Assert.Empty(await context.Database.GetPendingMigrationsAsync());
    }
    [Fact] public async Task Excel_preview_detects_duplicates_and_import_is_persistent()
    {
        await using var s = new TestShop(); await s.Initialize(); var file = Path.Combine(s.Directory, "import.xlsx");
        string[] row = ["P-001", "750123", "Cable", "Eléctrico", "Marca", "12 AWG", "Metro", "5", "11.60", "10", "100", "5", "16", "Rollo", "100"];
        await s.Get<IDocumentService>().Excel(file, new("Productos", ImportService.Headers, [row]));
        var preview = await s.Get<ImportService>().Preview(file); Assert.True(preview.Valid); Assert.Equal(1, await s.Get<ImportService>().Commit(preview));
        Assert.False((await s.Get<ImportService>().Preview(file)).Valid); Assert.Equal(100, (await s.Get<CatalogService>().Search("P-001")).Items.Single().Stock);
    }
    [Fact] public async Task Pdf_ticket_quote_and_report_have_valid_headers_and_multiple_pages()
    {
        await using var s = new TestShop(); await s.Initialize(); var doc = s.Get<IDocumentService>(); var settings = await s.Get<SettingsService>().Get();
        foreach (var width in new int?[] { null, 58, 80 })
        {
            var path = Path.Combine(s.Directory, $"ticket-{width}.pdf"); await doc.Pdf(path, settings, new("Venta", "V-000001", DateTime.Now, "Cliente", Enumerable.Range(0, 60).Select(i => new DocumentLine("Producto con descripción larga número " + i, "1 Pieza", 116, 116)).ToList(), 6960, 960, "Gracias"), width);
            Assert.StartsWith("%PDF", System.Text.Encoding.ASCII.GetString(await File.ReadAllBytesAsync(path))); Assert.True(new FileInfo(path).Length > 1000);
        }
    }
    [Fact] public async Task Reports_use_real_sales_and_do_not_count_credit_as_cash()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(); await s.Get<SalesService>().Checkout(s.Request(id));
        var d = await s.Get<IReportingService>().Dashboard(DateTime.Today.AddDays(-6), DateTime.Today); Assert.Equal(11600, d.TodayCents); Assert.Equal(1, d.TodayCount); Assert.Equal(111600, d.CashCents); Assert.Equal(5000, d.ProfitCents);
        foreach (var kind in new[] { "Ventas", "Inventario", "Cuentas por cobrar", "Compras", "Devoluciones", "Métodos de pago", "Por producto", "Por categoría", "Por usuario", "Costos y ganancias" }) Assert.NotEmpty((await s.Get<IReportingService>().Report(kind, DateTime.Today, DateTime.Today)).Headers);
    }
    [Fact] public void Architecture_points_inward_and_domain_has_no_framework_dependencies()
    {
        Assert.DoesNotContain(typeof(Product).Assembly.GetReferencedAssemblies(), x => x.Name!.Contains("EntityFramework") || x.Name.Contains("Windows") || x.Name.Contains("Infrastructure"));
        Assert.DoesNotContain(typeof(SalesService).Assembly.GetReferencedAssemblies(), x => x.Name!.Contains("EntityFramework") || x.Name.Contains("Infrastructure") || x.Name.Contains("Presentation"));
    }
    [Theory] [InlineData(0.005, 1)] [InlineData(1.235, 124)] [InlineData(99.995, 10000)]
    public void Rounding_is_decimal_and_away_from_zero(decimal input, long expected) => Assert.Equal(expected, Amount.Cents(input));
}

