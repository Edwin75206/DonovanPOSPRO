using System.Diagnostics;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Infrastructure.Persistence;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Infrastructure;
using Microsoft.EntityFrameworkCore.Migrations;
using Xunit;
namespace DonovanPOS.Tests;
public sealed class PerfectionTests
{
    [Theory] [InlineData(58)] [InlineData(80)] public async Task Thermal_pdf_keeps_printed_text_inside_final_page(int millimeters)
    {
        await using var s = new TestShop(); await s.Initialize();
        var path = Path.Combine(s.Directory, "ticket.pdf");
        await s.Get<IDocumentService>().Pdf(path, await s.Get<SettingsService>().Get(), new("Venta", "V-00001", DateTime.Today, "Público en general", [new("Tornillo galvanizado 1/4", "2 pzas", 58, 116)], 116, 16, "Pago en efectivo"), millimeters);
        using var pdf = PdfSharp.Pdf.IO.PdfReader.Open(path); var page = pdf.Pages[0];
        Assert.Equal(millimeters * 72d / 25.4, page.Width.Point, 2); Assert.InRange(page.Height.Point, 150, 800);
        var content = System.Text.Encoding.ASCII.GetString(page.Contents.CreateSingleContent().Stream.UnfilteredValue);
        var positions = System.Text.RegularExpressions.Regex.Matches(content, @"(?m)^(-?[\d.]+) (-?[\d.]+) Td$");
        Assert.NotEmpty(positions); double x = 0, y = 0;
        foreach (System.Text.RegularExpressions.Match position in positions)
        {
            x += double.Parse(position.Groups[1].Value, System.Globalization.CultureInfo.InvariantCulture); y += double.Parse(position.Groups[2].Value, System.Globalization.CultureInfo.InvariantCulture);
            Assert.InRange(x, 0, page.Width.Point); Assert.InRange(y, 0, page.Height.Point);
        }
        Assert.Contains("Tj", content);
    }
    [Fact] public async Task Barcode_labels_paginate_and_reject_unencodable_codes()
    {
        await using var s = new TestShop(); await s.Initialize(); var config = await s.Get<SettingsService>().Get(); var path = Path.Combine(s.Directory, "labels.pdf");
        await s.Get<IDocumentService>().Labels(path, config, [new("7501234567890", "Tornillo", 1250, 17)]);
        using var pdf = PdfSharp.Pdf.IO.PdfReader.Open(path); Assert.Equal(2, pdf.PageCount);
        await Assert.ThrowsAsync<RuleException>(() => s.Get<IDocumentService>().Labels(path, config, [new("NO-Ñ", "Tornillo", 1250, 1)]));
    }
    [Fact] public async Task Embedded_logo_survives_backup_and_restore_without_original_file()
    {
        await using var s = new TestShop(); await s.Initialize();
        var config = await s.Get<SettingsService>().Get(); config.LogoData = Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg=="); await s.Get<SettingsService>().Save(config);
        var backup = await s.Get<IBackupService>().Backup(Path.Combine(s.Directory, "copies")); config.LogoData = null; await s.Get<SettingsService>().Save(config);
        await s.Get<IBackupService>().Restore(backup); await s.Get<AuthService>().Login("admin", "Testing-only-2026");
        Assert.Equal(70, (await s.Get<SettingsService>().Get()).LogoData!.Length);
    }
    [Fact] public async Task Search_matches_accents_multiple_tokens_measure_brand_alias_and_updates_index()
    {
        await using var s = new TestShop(); await s.Initialize();
        var id = await s.Get<CatalogService>().Save(new Product { Sku = "TOR-14", Barcode = "750000004", Name = "Tornillo galvanizado", Category = "Tornillería", Brand = "Truper", Size = "1/4 x 2", Aliases = "pija taquete metálico", PriceCents = 250, StockUnits = 100000 });
        foreach (var term in new[] { "tornillo 1/4", "TORNILLERIA", "metalico truper", "750000004", "TOR-14" }) Assert.Equal(id, (await s.Get<CatalogService>().Search(term)).Items.Single().Id);
        Assert.Empty((await s.Get<CatalogService>().Search("tornillo 3/8")).Items);
        var product = (await s.Get<CatalogService>().Get(id))!; product.Aliases = "perno"; await s.Get<CatalogService>().Save(product);
        Assert.Empty((await s.Get<CatalogService>().Search("taquete")).Items); Assert.Single((await s.Get<CatalogService>().Search("perno 1/4")).Items);
        await s.Get<IDataStore>().Write(async db => { for (var i = 0; i < 2000; i++) db.Add(new Product { Sku = "LOAD-" + i, Name = "Producto grande " + i, Category = "Catálogo", PriceCents = 100 }); await db.Flush(); return true; });
        var timer = Stopwatch.StartNew(); Assert.Single((await s.Get<CatalogService>().Search("tornillo 1/4")).Items); Assert.True(timer.Elapsed < TimeSpan.FromSeconds(5));
        Assert.Equal(2000, (await s.Get<CatalogService>().Search(category: "Catálogo")).Total); Assert.Equal(100, (await s.Get<CatalogService>().Search(category: "Catálogo")).Items.Count);
    }
    [Fact] public async Task Suspended_sale_survives_reopen_without_stock_effect_and_can_only_be_charged_once()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product();
        var held = await s.Get<HeldSalesService>().Save(null, "Juan vuelve por la tarde", [new(id, 2)], null, 0);
        Assert.Equal(20, (await s.Get<CatalogService>().Get(id))!.Stock);
        var p = (await s.Get<CatalogService>().Get(id))!; p.PriceCents = 12000; await s.Get<CatalogService>().Save(p);
        var detail = await s.Get<HeldSalesService>().Detail(held.Id); Assert.Contains(detail.Notices, x => x.Contains("precio"));
        var fresh = new Database(s.Directory); await fresh.Initialize(); Assert.Equal(1, await fresh.Read(db => db.Count<HeldSale>()));
        var sale = await s.Get<SalesService>().Checkout(s.Request(id, 2, 240) with { HeldSaleId = held.Id }); Assert.Equal(24000, sale.TotalCents);
        Assert.Empty(await s.Get<HeldSalesService>().List()); Assert.Equal(18, (await s.Get<CatalogService>().Get(id))!.Stock);
        await Assert.ThrowsAsync<RuleException>(() => s.Get<SalesService>().Checkout(s.Request(id, 2, 240) with { HeldSaleId = held.Id }));
    }
    [Fact] public async Task Suspended_sale_detects_reduced_stock_and_final_checkout_rolls_back()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(stock: 2);
        var held = await s.Get<HeldSalesService>().Save(null, "En espera", [new(id, 2)], null, 0); await s.Get<SalesService>().Checkout(s.Request(id));
        Assert.Contains((await s.Get<HeldSalesService>().Detail(held.Id)).Notices, x => x.Contains("insuficientes"));
        await Assert.ThrowsAsync<RuleException>(() => s.Get<SalesService>().Checkout(s.Request(id, 2, 232) with { HeldSaleId = held.Id }));
        Assert.Single(await s.Get<HeldSalesService>().List()); Assert.Equal(1, (await s.Get<CatalogService>().Get(id))!.Stock);
    }
    [Fact] public async Task Tiny_discounted_lines_allocate_total_and_tax_without_negative_lines()
    {
        await using var s = new TestShop(); await s.Initialize(); List<SaleItem> items = [];
        for (var i = 0; i < 10; i++) items.Add(new(await s.Product(.01m, 1), 1));
        var sale = await s.Get<SalesService>().Checkout(new(Guid.NewGuid().ToString(), items, [new(PayMethod.Efectivo, .06m)], .06m, 40));
        var detail = await s.Get<SalesService>().Detail(sale.Id); Assert.Equal(6, sale.TotalCents); Assert.Equal(1, sale.TaxCents);
        Assert.Equal(sale.TotalCents, detail.Lines.Sum(x => x.TotalCents)); Assert.Equal(sale.TaxCents, detail.Lines.Sum(x => x.TaxCents)); Assert.All(detail.Lines, x => { Assert.InRange(x.TotalCents, 0, 1); Assert.InRange(x.TaxCents, 0, x.TotalCents); });
    }
    [Fact] public async Task Automatic_backup_runs_when_due_and_records_integrity_and_hash()
    {
        await using var s = new TestShop(); await s.Initialize(); var config = await s.Get<SettingsService>().Get(); config.BackupDirectory = Path.Combine(s.Directory, "automatic"); await s.Get<SettingsService>().Save(config);
        var backup = await s.Get<IBackupService>().BackupIfDue(); Assert.NotNull(backup); Assert.True(File.Exists(backup)); Assert.Null(await s.Get<IBackupService>().BackupIfDue());
        var log = (await s.Get<IBackupService>().History()).Single(); Assert.True(log.Successful); Assert.Equal(64, log.Sha256.Length); Assert.True(log.FileBytes > 0);
    }
    [Fact] public async Task Invalid_restore_does_not_replace_existing_sales()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product(); await s.Get<SalesService>().Checkout(s.Request(id));
        var invalid = Path.Combine(s.Directory, "not-a-backup.db"); await File.WriteAllTextAsync(invalid, "not sqlite");
        await Assert.ThrowsAnyAsync<Exception>(() => s.Get<IBackupService>().Restore(invalid));
        Assert.Equal(1, await s.Get<IDataStore>().Read(db => db.Count<Sale>())); Assert.Equal(19, (await s.Get<CatalogService>().Get(id))!.Stock);
    }
    [Fact] public async Task Failed_import_preserves_preview_for_corrected_retry()
    {
        await using var s = new TestShop(); await s.Initialize();
        var a = new Product { Sku = "IMPORT-A", Name = "Primero", PriceCents = 100, StockUnits = 20000 }; var b = new Product { Sku = "IMPORT-B", Name = "Segundo", PriceCents = 100, StockUnits = 30000 };
        var preview = new ImportPreview([new(2, a, ""), new(3, b, "")]); var existing = await s.Get<CatalogService>().Save(b.CopyForImport());
        await Assert.ThrowsAsync<RuleException>(() => s.Get<ImportService>().Commit(preview)); Assert.Equal(0, a.Id); Assert.Equal(20000, a.StockUnits); Assert.Empty((await s.Get<CatalogService>().Search("IMPORT-A")).Items);
        var p = (await s.Get<CatalogService>().Get(existing))!; p.Sku = "RENAMED"; await s.Get<CatalogService>().Save(p);
        Assert.Equal(2, await s.Get<ImportService>().Commit(preview)); Assert.Equal(2, (await s.Get<CatalogService>().Search("IMPORT-A")).Items.Single().Stock);
    }
    [Fact] public async Task Existing_installation_upgrades_with_safety_copy_and_rebuilt_search_index()
    {
        await using var s = new TestShop(); System.IO.Directory.CreateDirectory(s.Directory);
        await using (var db = s.Get<Database>().Context())
        {
            await db.GetService<IMigrator>().MigrateAsync("20261030212631_InitialCommerce");
            await db.Database.ExecuteSqlRawAsync("INSERT INTO Product(Sku,Name,Category,Brand,Size,Unit,PriceCents,WholesaleCents,StockUnits,MinimumUnits,TaxBasisPoints,PackName,PackUnits,Active) VALUES('ANT-01','Tornillo anterior','Tornillería','','1/4',0,100,0,10000,0,1600,'Caja',10000,1)");
        }
        await s.Get<Database>().Initialize(); Assert.Single(System.IO.Directory.GetFiles(Path.Combine(s.Directory, "antes-de-actualizar"), "*.db"));
        await s.Get<AuthService>().Setup("Migrada", "admin", "Administrador", "Testing-only-2026"); await s.Get<AuthService>().Login("admin", "Testing-only-2026"); Assert.Single((await s.Get<CatalogService>().Search("tornilleria 1/4")).Items);
    }
    [Fact] public async Task Complete_workday_reconciles_cash_stock_credit_returns_reports_backup_and_restart()
    {
        await using var s = new TestShop(); await s.Initialize(); var id = await s.Product();
        var supplier = await s.Get<PurchasesService>().SaveSupplier(new Supplier { Name = "Abastecedor del día" });
        await s.Get<PurchasesService>().Receive(supplier, "DIA-001", DateTime.Today, [new(id, 2, 50)], true);
        await s.Get<SalesService>().Checkout(s.Request(id, 2, 232) with { Payments = [new(PayMethod.Efectivo, 100), new(PayMethod.Transferencia, 132)], Tendered = 200 });
        var held = await s.Get<HeldSalesService>().Save(null, "Cliente regresa", [new(id, 2)], null, 0); await s.Get<SalesService>().Checkout(s.Request(id, 2, 232) with { HeldSaleId = held.Id });
        var quote = await s.Get<QuotesService>().Save(0, [new(id, 1)], null, 10, DateTime.Today.AddDays(1), "Prueba de jornada"); await s.Get<SalesService>().Checkout(s.Request(id, 1, 104.4m) with { QuoteId = quote.Id });
        var customer = await s.Get<CustomersService>().Save(new Customer { Name = "Cliente del día", CreditLimitCents = 20000 });
        var credit = await s.Get<SalesService>().Checkout(s.Request(id) with { CustomerId = customer, Payments = [new(PayMethod.Credito, 116)], Tendered = 0, DueDate = DateTime.Today.AddDays(15) });
        await s.Get<CustomersService>().Pay(customer, credit.Id, 50, PayMethod.Efectivo, "Abono del día"); await s.Get<ReturnService>().Return(credit.Id, [], "Cancelación de compra a crédito", PayMethod.Efectivo, true);
        await s.Get<CashService>().Movement(-36.4m, "Gasto de entrega"); Assert.Equal(130000, (await s.Get<CashService>().Overview()).ExpectedCents); Assert.Equal(17, (await s.Get<CatalogService>().Get(id))!.Stock); Assert.Equal(0, (await s.Get<CustomersService>().Balances()).Single().BalanceCents);
        var dashboard = await s.Get<IReportingService>().Dashboard(DateTime.Today, DateTime.Today); Assert.Equal(56840, dashboard.TodayCents);
        var close = await s.Get<CashService>().Close(1300, "Corte cuadrado"); Assert.Equal(close.ExpectedCents, close.CountedCents);
        var backup = await s.Get<IBackupService>().Backup(Path.Combine(s.Directory, "closing")); await s.Get<IBackupService>().Restore(backup); await s.Get<AuthService>().Login("admin", "Testing-only-2026");
        var fresh = new Database(s.Directory); await fresh.Initialize(); Assert.Equal(4, await fresh.Read(db => db.Count<Sale>())); Assert.Equal(17, (await s.Get<CatalogService>().Get(id))!.Stock); Assert.Null((await s.Get<CashService>().Overview()).Session); Assert.Contains(await s.Get<SettingsService>().Audit(), x => x.Action == "Restaurar respaldo");
    }
}
