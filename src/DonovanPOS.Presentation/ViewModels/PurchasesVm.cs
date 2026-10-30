using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed record PurchaseDraft(Product Product, decimal Quantity, decimal Cost, bool ByPack)
{
    public string Name => Product.Name;
    public string Unit => ByPack ? Product.PackName : Product.Unit.ToString();
    public decimal Total => Amount.Round(Quantity * Cost);
}
public sealed partial class PurchasesVm(PurchasesService purchases, CatalogService catalog, Dialogs dialogs, IClock clock, UiState ui) : PageVm(ui)
{
    public override string Title => "Proveedores y compras";
    public ObservableCollection<Supplier> Suppliers { get; } = [];
    public ObservableCollection<Product> Products { get; } = [];
    public ObservableCollection<PurchaseDraft> Lines { get; } = [];
    [ObservableProperty] private Supplier? selectedSupplier;
    [ObservableProperty] private Product? selectedProduct;
    [ObservableProperty] private PurchaseDraft? selectedLine;
    [ObservableProperty] private string search = "";
    [ObservableProperty] private string quantity = "1";
    [ObservableProperty] private string cost = "0";
    [ObservableProperty] private bool byPack;
    public decimal Total => Lines.Sum(x => x.Total);
    public string PackLabel => SelectedProduct is null ? "Recibir por presentación" : $"Recibir por {SelectedProduct.PackName} ({Amount.Quantity(SelectedProduct.PackUnits):0.####} {SelectedProduct.Unit})";
    public override Task Load() => Run(async () => { Suppliers.Clear(); foreach (var s in await purchases.Suppliers()) Suppliers.Add(s); SelectedSupplier ??= Suppliers.FirstOrDefault(); await SearchProducts(); });
    partial void OnSelectedProductChanged(Product? value) { UpdateCost(); OnPropertyChanged(nameof(PackLabel)); }
    partial void OnByPackChanged(bool value) => UpdateCost();
    private void UpdateCost() { if (SelectedProduct is not null) Cost = (Amount.Pesos(SelectedProduct.CostCents ?? 0) * (ByPack ? Amount.Quantity(SelectedProduct.PackUnits) : 1)).ToString("0.00", Mx.Culture); }
    [RelayCommand] private Task SearchCatalog() => Run(SearchProducts);
    private async Task SearchProducts() { Products.Clear(); foreach (var p in (await catalog.Search(Search)).Items) Products.Add(p); SelectedProduct = Products.FirstOrDefault(); }
    [RelayCommand] private Task Add() => Run(() =>
    {
        var p = SelectedProduct ?? throw new RuleException("Selecciona un producto."); var qty = Mx.Number(Quantity); var cost = Mx.Number(Cost); Amount.Require(qty > 0 && cost >= 0, "Revisa cantidad y costo.");
        Lines.Add(new(p, qty, cost, ByPack)); Quantity = "1"; OnPropertyChanged(nameof(Total)); return Task.CompletedTask;
    });
    [RelayCommand] private void Remove() { if (SelectedLine is not null) { Lines.Remove(SelectedLine); OnPropertyChanged(nameof(Total)); } }
    [RelayCommand] private async Task SupplierNew() => await SupplierEdit(null);
    [RelayCommand] private async Task SupplierChange() { if (SelectedSupplier is not null) await SupplierEdit(SelectedSupplier); }
    private async Task SupplierEdit(Supplier? input)
    {
        var s = input ?? new Supplier();
        if (await dialogs.Form(s.Id == 0 ? "Agregar proveedor" : "Editar proveedor", "Datos para identificar tus compras y entregas.", [new("name", "Nombre", s.Name), new("phone", "Teléfono", s.Phone), new("email", "Correo", s.Email), new("address", "Dirección", s.Address)], async f => { await purchases.SaveSupplier(new Supplier { Id = s.Id, Name = f["name"], Phone = f["phone"], Email = f["email"], Address = f["address"] }); })) await Load();
    }
    [RelayCommand] private async Task Receive()
    {
        if (SelectedSupplier is null || Lines.Count == 0) { Ui.Notify("Selecciona un proveedor y agrega productos.", true); return; }
        if (await dialogs.Form("Recibir compra", $"{SelectedSupplier.Name} · {Lines.Count} partidas · costo neto {Total:C2}. Las existencias se agregarán al confirmar.", [new("ref", "Folio del proveedor"), new("date", "Fecha de compra (dd/MM/aaaa)", clock.Now.ToString("dd/MM/yyyy")), new("cash", "¿Pagar con efectivo de esta caja?", "No", Dialogs.YesNo)], async f => { var p = await purchases.Receive(SelectedSupplier.Id, f["ref"], DateTime.ParseExact(f["date"], "dd/MM/yyyy", Mx.Culture), Lines.Select(x => new PurchaseItem(x.Product.Id, x.Quantity, x.Cost, x.ByPack)).ToList(), f.Boolean("cash")); Ui.Notify("Compra " + p.Folio + " recibida."); }, "Confirmar recepción")) { Lines.Clear(); OnPropertyChanged(nameof(Total)); await Load(); }
    }
    [RelayCommand] private Task History() => Run(async () => { var rows = await purchases.History(); dialogs.Table(new("Historial de compras", ["Folio", "Referencia proveedor", "Fecha", "Costo neto"], rows.Select(x => new[] { x.Folio, x.SupplierReference, x.CreatedAt.ToString("dd/MM/yyyy"), Mx.Money(x.TotalCents) }).ToList())); });
}
