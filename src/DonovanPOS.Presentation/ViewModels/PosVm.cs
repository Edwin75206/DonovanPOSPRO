using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Domain.Pricing;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class CartRow : ObservableObject
{
    public required Product Product { get; init; }
    [ObservableProperty] private decimal quantity = 1;
    public string Name => Product.Name;
    public string UnitLabel => Product.Unit.ToString();
    public long? LockedCents { get; set; }
    public decimal Total => LockedCents is long c ? Amount.Pesos(c) : Amount.Round(Quantity * Product.UnitPrice);
    partial void OnQuantityChanged(decimal value) => OnPropertyChanged(nameof(Total));
}
public sealed partial class PosVm(CatalogService catalog, SalesService sales, QuotesService quotes, CustomersService customers, SettingsService settings, IDocumentService documents, Dialogs dialogs, UserSession session, IClock clock, UiState ui) : PageVm(ui)
{
    public override string Title => "Punto de venta";
    public ObservableCollection<Product> Products { get; } = [];
    public ObservableCollection<CartRow> Cart { get; } = [];
    public ObservableCollection<Customer> Customers { get; } = [];
    [ObservableProperty] private Product? selectedProduct;
    [ObservableProperty] private CartRow? selectedLine;
    [ObservableProperty] private Customer? selectedCustomer;
    [ObservableProperty] private string search = "";
    [ObservableProperty] private string quantityText = "1";
    [ObservableProperty] private decimal discountPercent;
    [ObservableProperty] private string resultText = "";
    [ObservableProperty] private bool quoteLocked;
    [ObservableProperty] private string quoteLabel = "Venta de mostrador";
    [ObservableProperty] private string customerLabel = "Público en general";
    [ObservableProperty] private bool hasReceipt;
    [ObservableProperty] private string cashLabel = "";
    private Sale? lastSale;
    private int? quoteId;
    private int editingQuoteId;
    private string requestId = Guid.NewGuid().ToString();
    private CancellationTokenSource? debounce;
    public bool IsEmpty => Cart.Count == 0;
    public decimal Total => QuoteLocked ? Cart.Sum(x => x.Total) : Calculate().GrossTotal;
    public decimal Tax => QuoteLocked ? Cart.Sum(x => Amount.Round(x.Total - x.Total / (1 + x.Product.VatRate))) : Calculate().VatTotal;
    public decimal Subtotal => Total - Tax;
    public string CountLabel => $"{Cart.Count} productos";
    public bool CanDiscount => session.Can(Permission.Discount);
    public Action? FocusSearch { get; set; }
    private SaleSummary Calculate()
    {
        var cart = new DonovanPOS.Domain.Pricing.Cart(); foreach (var row in Cart) cart.AddOrIncrement(row.Product, row.Quantity);
        if (DiscountPercent != 0) cart.ApplyCartDiscount(new PercentageDiscount(DiscountPercent / 100m)); return PricingService.Summarize(cart);
    }
    public override Task Load() => Run(async () => { await SearchNow(); Customers.Clear(); Customers.Add(new Customer { Id = 0, Name = "Público en general" }); foreach (var c in await customers.List()) Customers.Add(c); SelectedCustomer ??= Customers.First(); FocusSearch?.Invoke(); });
    partial void OnSearchChanged(string value) { _ = SearchDelayed(); }
    partial void OnDiscountPercentChanged(decimal value) => TotalsChanged();
    partial void OnSelectedCustomerChanged(Customer? value) { CustomerLabel = value?.Name ?? "Público en general"; }
    private async Task SearchDelayed()
    {
        debounce?.Cancel(); debounce = new(); var ct = debounce.Token;
        try { await Task.Delay(150, ct); if (!ct.IsCancellationRequested) await SearchNow(); } catch (OperationCanceledException) { } catch (Exception ex) { Ui.Notify(Ui.Error(ex), true); }
    }
    private async Task SearchNow()
    {
        var term = Search; var page = await catalog.Search(term); if (term != Search) return;
        Products.Clear(); foreach (var p in page.Items) Products.Add(p); SelectedProduct = Products.FirstOrDefault();
    }
    private void TotalsChanged() { foreach (var property in new[] { nameof(Total), nameof(Tax), nameof(Subtotal), nameof(CountLabel), nameof(IsEmpty) }) OnPropertyChanged(property); }
    [RelayCommand] private Task Add() => Run(async () =>
    {
        if (QuoteLocked) throw new RuleException("Esta venta conserva una cotización. Inicia otra venta para cambiar sus partidas.");
        var results = await catalog.Search(Search);
        var exact = results.Items.FirstOrDefault(p => p.Barcode == Search.Trim() || p.Sku.Equals(Search.Trim(), StringComparison.OrdinalIgnoreCase));
        var p = exact ?? SelectedProduct ?? results.Items.FirstOrDefault() ?? throw new RuleException("No se encontró el producto. Revisa la clave o el código escaneado.");
        var quantity = Mx.Number(QuantityText); Amount.QuantityFor(p.Unit, Amount.Units(quantity));
        var row = Cart.FirstOrDefault(x => x.Product.Id == p.Id);
        Amount.Require((row?.Quantity ?? 0) + quantity <= p.Stock, $"Existencia disponible: {p.StockLabel}.");
        if (row is null) { row = new CartRow { Product = p, Quantity = quantity }; row.PropertyChanged += (_, _) => TotalsChanged(); Cart.Add(row); } else row.Quantity += quantity;
        SelectedLine = row; Search = ""; QuantityText = "1"; ResultText = ""; TotalsChanged(); FocusSearch?.Invoke();
    });
    [RelayCommand] private async Task ChangeQuantity()
    {
        if (SelectedLine is null || QuoteLocked) return; var row = SelectedLine;
        await dialogs.Form("Cambiar cantidad", row.Name + " · " + row.UnitLabel, [new("qty", "Cantidad", row.Quantity.ToString("0.####", Mx.Culture))], f =>
        {
            var qty = f.Number("qty"); Amount.QuantityFor(row.Product.Unit, Amount.Units(qty)); Amount.Require(qty <= row.Product.Stock, "La cantidad supera las existencias disponibles."); row.Quantity = qty; TotalsChanged(); return Task.CompletedTask;
        }); FocusSearch?.Invoke();
    }
    [RelayCommand] private void Remove() { if (SelectedLine is not null && !QuoteLocked) { Cart.Remove(SelectedLine); TotalsChanged(); FocusSearch?.Invoke(); } }
    [RelayCommand] private async Task Discount()
    {
        if (QuoteLocked) return;
        await dialogs.Form("Descuento autorizado", "El descuento se distribuye entre las partidas antes de calcular el IVA incluido.", [new("discount", "Descuento %", DiscountPercent.ToString("0.##", Mx.Culture))], f => { session.Require(Permission.Discount); var value = f.Number("discount"); Amount.Require(value is >= 0 and <= 100, "Usa un porcentaje entre 0 y 100."); DiscountPercent = value; return Task.CompletedTask; });
    }
    [RelayCommand] private async Task Clear()
    {
        if (Cart.Count == 0 || await dialogs.Confirm("Iniciar otra venta", "Se vaciarán las partidas que aún no has cobrado.", () => Task.CompletedTask)) ClearCart();
    }
    private void ClearCart()
    {
        Cart.Clear(); DiscountPercent = 0; quoteId = null; editingQuoteId = 0; QuoteLocked = false; QuoteLabel = "Venta de mostrador"; requestId = Guid.NewGuid().ToString(); SelectedCustomer = Customers.FirstOrDefault(x => x.Id == 0); TotalsChanged(); FocusSearch?.Invoke();
    }
    [RelayCommand] private async Task Checkout()
    {
        if (Busy || IsEmpty) return;
        var total = Total;
        var confirmed = await dialogs.Form("Cobrar " + total.ToString("C2", Mx.Culture), CustomerLabel + ". Tarjeta y transferencia registran pagos recibidos; verifica el comprobante.",
        [new("cash", "Pago en efectivo", total.ToString("0.00", Mx.Culture)), new("received", "Efectivo recibido", total.ToString("0.00", Mx.Culture)), new("card", "Pago con tarjeta", "0"), new("transfer", "Pago por transferencia", "0"), new("credit", "A crédito al cliente", "0"), new("due", "Fecha de pago del crédito (dd/MM/aaaa)", clock.Now.AddDays(30).ToString("dd/MM/yyyy")), new("ref", "Referencia de tarjeta / transferencia")], async f =>
        {
            var cash = f.Number("cash"); var credit = f.Number("credit");
            var payments = new[] { new PaymentInput(PayMethod.Efectivo, cash), new PaymentInput(PayMethod.Tarjeta, f.Number("card"), f["ref"]), new PaymentInput(PayMethod.Transferencia, f.Number("transfer"), f["ref"]), new PaymentInput(PayMethod.Credito, credit) }.Where(x => x.Amount != 0 || total == 0 && x.Method == PayMethod.Efectivo).ToArray();
            DateTime? due = credit > 0 ? DateTime.ParseExact(f["due"], "dd/MM/yyyy", Mx.Culture) : null;
            lastSale = await sales.Checkout(new(requestId, Cart.Select(x => new SaleItem(x.Product.Id, x.Quantity)).ToList(), payments, cash > 0 ? f.Number("received") : 0, QuoteLocked ? 0 : DiscountPercent, SelectedCustomer?.Id is > 0 ? SelectedCustomer.Id : null, due, quoteId));
        }, "Registrar venta", f =>
        {
            var paid = f.Number("cash") + f.Number("card") + f.Number("transfer") + f.Number("credit");
            return $"Por cubrir: {(total - paid).ToString("C2", Mx.Culture)}   ·   Cambio: {Math.Max(0, f.Number("received") - f.Number("cash")).ToString("C2", Mx.Culture)}";
        });
        if (confirmed && lastSale is not null)
        {
            ResultText = $"Venta {lastSale.Folio} registrada · {Mx.Money(lastSale.TotalCents)} · Cambio {Mx.Money(lastSale.ChangeCents)}"; HasReceipt = true; ClearCart(); await SearchNow(); Ui.Notify(ResultText);
        }
    }
    [RelayCommand] private async Task SaveQuote()
    {
        if (IsEmpty || QuoteLocked) return;
        var saved = await dialogs.Form(editingQuoteId == 0 ? "Guardar cotización" : "Actualizar cotización", $"{Total:C2} · las existencias no se descontarán.", [new("date", "Vigencia (dd/MM/aaaa)", clock.Now.AddDays(15).ToString("dd/MM/yyyy")), new("notes", "Observaciones", Multiline: true)], async f =>
        {
            var quote = await quotes.Save(editingQuoteId, Cart.Select(x => new SaleItem(x.Product.Id, x.Quantity)).ToList(), SelectedCustomer?.Id is > 0 ? SelectedCustomer.Id : null, DiscountPercent, DateTime.ParseExact(f["date"], "dd/MM/yyyy", Mx.Culture), f["notes"]); Ui.Notify("Cotización " + quote.Folio + " guardada.");
        }); if (saved) ClearCart();
    }
    public async Task LoadQuote(QuoteDetail detail, bool edit)
    {
        if (!IsEmpty && !await dialogs.Confirm("Abrir cotización", "La venta actual aún no se ha cobrado. Se reemplazará por la cotización seleccionada.", () => Task.CompletedTask)) return;
        ClearCart(); await Load();
        foreach (var line in detail.Lines)
        {
            var p = await catalog.Get(line.ProductId) ?? throw new RuleException("Producto no encontrado.");
            if (!edit) { p.PriceCents = line.PriceCents; p.TaxBasisPoints = line.TaxBasisPoints; }
            var row = new CartRow { Product = p, Quantity = Amount.Quantity(line.QuantityUnits), LockedCents = edit ? null : line.TotalCents }; row.PropertyChanged += (_, _) => TotalsChanged(); Cart.Add(row);
        }
        quoteId = edit ? null : detail.Quote.Id; editingQuoteId = edit ? detail.Quote.Id : 0; QuoteLocked = !edit;
        QuoteLabel = detail.Quote.Folio + (edit ? " · edición con precios actuales" : " · precios acordados"); SelectedCustomer = Customers.FirstOrDefault(x => x.Id == detail.Quote.CustomerId) ?? Customers.FirstOrDefault(); TotalsChanged();
    }
    public static BusinessDocument Receipt(SaleDetail d) => new("Ticket de venta", d.Sale.Folio, d.Sale.CreatedAt, d.Sale.CustomerName, d.Lines.Select(x => new DocumentLine(x.Name, $"{Amount.Quantity(x.QuantityUnits):0.####} {x.Unit}", Amount.Pesos(x.PriceCents), Amount.Pesos(x.TotalCents))).ToList(), Amount.Pesos(d.Sale.TotalCents), Amount.Pesos(d.Sale.TaxCents), string.Join(" · ", d.Payments.Select(x => x.Method + ": " + Mx.Money(x.AmountCents))) + $"\nCambio: {Mx.Money(d.Sale.ChangeCents)}" + (d.Sale.Cancelled ? "\nVENTA CANCELADA / DEVUELTA" : "") + (d.Returns.Count > 0 ? "\nDevoluciones: " + Mx.Money(d.Returns.Sum(x => x.TotalCents)) : ""));
    [RelayCommand] private Task TicketPdf() => Run(async () => { if (lastSale is null) return; var path = dialogs.SavePath(lastSale.Folio, ".pdf"); if (path is null) return; var config = await settings.Get(); await documents.Pdf(path, config, Receipt(await sales.Detail(lastSale.Id)), config.TicketWidthMm); Mx.OpenFile(path); });
    [RelayCommand] private Task Print() => Run(async () => { if (lastSale is not null) dialogs.Print(await settings.Get(), Receipt(await sales.Detail(lastSale.Id))); });
}
