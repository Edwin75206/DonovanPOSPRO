using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;
public sealed partial class PosVm
{
    public ObservableCollection<string> Categories { get; } = ["Todos"];
    [ObservableProperty] private string selectedCategory = "Todos";
    [ObservableProperty] private string productCount = "";
    [ObservableProperty] private string heldCount = "Recuperar";
    public bool NoResults => Products.Count == 0;
    private int productPage;
    private int? heldId;
    private async Task RefreshCart()
    {
        if (QuoteLocked) return; var changed = false; List<string> notices = [];
        foreach (var row in Cart)
        {
            var p = await catalog.Get(row.Product.Id); if (p is null) continue;
            changed |= p.PriceCents != row.Product.PriceCents || p.WholesaleCents != row.Product.WholesaleCents;
            if (!p.Active) notices.Add(p.Name + ": archivado."); else if (p.Stock < row.Quantity) notices.Add(p.Name + ": ajusta la cantidad a la existencia disponible.");
            row.RefreshProduct(p);
        }
        TotalsChanged(); if (changed) notices.Add("El carrito se actualizó con los precios vigentes."); if (notices.Count > 0) Ui.Notify(string.Join("\n", notices));
    }
    partial void OnSelectedCategoryChanged(string value) { productPage = 0; _ = SearchDelayed(); }
    [RelayCommand] private Task NextProducts() => Run(async () => { if (Products.Count == 100) { productPage++; await SearchNow(); } });
    [RelayCommand] private Task PreviousProducts() => Run(async () => { if (productPage > 0) { productPage--; await SearchNow(); } });
    [RelayCommand] private Task AddProduct(Product? product) => Run(() => { if (product is not null) AddToCart(product); return Task.CompletedTask; });
    [RelayCommand] private Task Increase(CartRow? row) => Run(() => { if (row is not null && !QuoteLocked) { Amount.Require(row.Quantity + 1 <= row.Product.Stock, "No hay existencias suficientes para otra unidad."); row.Quantity++; SelectedLine = row; TotalsChanged(); } return Task.CompletedTask; });
    [RelayCommand] private void Decrease(CartRow? row) { if (row is null || QuoteLocked) return; if (row.Quantity <= 1) Cart.Remove(row); else row.Quantity--; SelectedLine = row; TotalsChanged(); }
    [RelayCommand] private void RemoveLine(CartRow? row) { if (row is null || QuoteLocked) return; Cart.Remove(row); TotalsChanged(); }
    [RelayCommand] private Task QuantityFor(CartRow? row) { SelectedLine = row; return ChangeQuantity(); }
    [RelayCommand] private Task Wholesale(CartRow? row) => Run(() => { session.Require(Permission.Discount); if (row is not null && !QuoteLocked) { Amount.Require(row.Product.WholesaleCents > 0, "Este producto aún no tiene precio de mayoreo."); row.Wholesale = !row.Wholesale; TotalsChanged(); } return Task.CompletedTask; });
    [RelayCommand] private void FindProduct() => FocusSearch?.Invoke();
    private async Task RefreshHeldCount() => HeldCount = "Recuperar (" + (await held.List()).Count + ")";
    [RelayCommand] private async Task Suspend()
    {
        if (IsEmpty || QuoteLocked) return;
        if (await dialogs.Form("Guardar para después", "La venta quedará disponible al reiniciar. Las existencias no se descuentan hasta cobrar.", [new("label", "Nombre o referencia", SelectedCustomer?.Id > 0 ? CustomerLabel : "Venta de mostrador")], async f =>
        {
            await held.Save(heldId, f["label"], Cart.Select(x => new SaleItem(x.Product.Id, x.Quantity, x.Wholesale)).ToList(), SelectedCustomer?.Id is > 0 ? SelectedCustomer.Id : null, DiscountPercent);
        }, "Suspender venta")) { ClearCart(); await RefreshHeldCount(); Ui.Notify("Venta suspendida. Puedes recuperarla cuando lo necesites."); }
    }
    [RelayCommand] private Task Recover() => Run(async () =>
    {
        var pending = await held.List(); Amount.Require(pending.Count > 0, "Aún no tienes ventas suspendidas.");
        if (!IsEmpty && !await dialogs.Confirm("Recuperar una venta", "La venta actual será reemplazada. Suspéndela primero si necesitas conservarla.", () => Task.CompletedTask)) return;
        await dialogs.Form("Ventas en espera", "Al recuperar, se revisan los precios y existencias actuales.", [new("held", "Selecciona una venta", pending[0].Id.ToString(), pending.Select(x => new Choice(x.Id.ToString(), $"{x.Folio} · {x.Label} · {x.CreatedAt:dd/MM HH:mm}")).ToArray())], async f =>
        {
            var detail = await held.Detail(f.Integer("held")); ClearCart(); heldId = detail.Sale.Id;
            foreach (var line in detail.Lines)
            {
                var p = await catalog.Get(line.ProductId) ?? throw new RuleException("No se encontró un producto."); var row = new CartRow { Product = p, Quantity = Amount.Quantity(line.QuantityUnits), Wholesale = line.Wholesale }; row.PropertyChanged += (_, _) => TotalsChanged(); Cart.Add(row);
            }
            DiscountPercent = detail.Sale.DiscountParts / 10000m; SelectedCustomer = Customers.FirstOrDefault(x => x.Id == detail.Sale.CustomerId) ?? Customers.FirstOrDefault(); QuoteLabel = detail.Sale.Folio + " · " + detail.Sale.Label; TotalsChanged();
            if (detail.Notices.Count > 0) Ui.Notify(string.Join("\n", detail.Notices), true); else Ui.Notify("Venta recuperada con precios y existencias vigentes.");
        }, "Recuperar venta");
    });
    [RelayCommand] private Task QuickPrice() => Run(async () =>
    {
        await dialogs.Form("Consultar precio", "Busca por clave, código, descripción, marca o medida. No se agregará nada a la venta.", [new("search", "Producto", Search)], async f =>
        {
            var result = await catalog.Search(f["search"]); dialogs.Table(new("Consulta de precios", ["Clave", "Producto", "Medida", "Precio con IVA", "Mayoreo", "Existencia"], result.Items.Select(x => new[] { x.Sku, x.Name, x.Size, Mx.Money(x.PriceCents), x.WholesaleCents > 0 ? Mx.Money(x.WholesaleCents) : "—", x.StockLabel }).ToList()));
        }, "Consultar");
    });
}
