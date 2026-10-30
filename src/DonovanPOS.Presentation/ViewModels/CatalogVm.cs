using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class CatalogVm(CatalogService catalog, ImportService import, SettingsService settings, IDocumentService documents, Dialogs dialogs, UiState ui, bool inventory) : PageVm(ui)
{
    public bool Inventory => inventory;
    public override string Title => Inventory ? "Inventario" : "Productos";
    public string Subtitle => Inventory ? "Cada entrada y salida, con un origen claro." : "Un catálogo ordenado para vender más rápido.";
    public ObservableCollection<Product> Items { get; } = [];
    [ObservableProperty] private Product? selected;
    [ObservableProperty] private string search = "";
    [ObservableProperty] private string countLabel = "";
    [ObservableProperty] private bool showArchived;
    private int page;
    private CancellationTokenSource? debounce;
    partial void OnSearchChanged(string value) { page = 0; _ = Debounce(); }
    partial void OnShowArchivedChanged(bool value) { page = 0; _ = Refresh(); }
    private async Task Debounce()
    {
        debounce?.Cancel(); debounce = new(); var ct = debounce.Token;
        try { await Task.Delay(180, ct); if (!ct.IsCancellationRequested) await Refresh(); } catch (OperationCanceledException) { }
    }
    public override Task Load() => Refresh();
    [RelayCommand] private Task Refresh() => Run(async () => { var result = await catalog.Search(Search, page, ShowArchived); Items.Clear(); foreach (var p in result.Items) Items.Add(p); CountLabel = $"{result.Total:N0} productos · página {page + 1} · hasta 100 por página"; });
    [RelayCommand] private async Task Next() { if (Items.Count == 100) { page++; await Refresh(); } }
    [RelayCommand] private async Task Previous() { if (page > 0) { page--; await Refresh(); } }
    [RelayCommand] private Task New() => EditProduct(null);
    [RelayCommand] private Task Edit() => Selected is null ? Task.CompletedTask : EditProduct(Selected);
    private async Task EditProduct(Product? product)
    {
        var config = await settings.Get(); var p = product ?? new Product { TaxBasisPoints = config.DefaultTaxBasisPoints };
        string N(decimal value) => value.ToString("0.####", Mx.Culture);
        var saved = await dialogs.Form(product is null ? "Agregar producto" : "Editar producto", "Los precios de venta incluyen IVA. La existencia se conserva en la unidad base; registra sus cambios en Inventario.",
        [new("sku", "Clave interna", p.Sku), new("barcode", "Código de barras", p.Barcode ?? ""), new("name", "Descripción", p.Name), new("category", "Categoría", p.Category), new("brand", "Marca", p.Brand), new("size", "Medidas", p.Size), new("unit", "Unidad base de venta", p.Unit.ToString(), Dialogs.Options<Unit>()), new("cost", "Costo neto por unidad (vacío si falta)", p.CostCents is long cost ? N(Amount.Pesos(cost)) : ""), new("price", "Precio de venta con IVA", N(p.UnitPrice)), new("wholesale", "Precio de mayoreo con IVA", N(Amount.Pesos(p.WholesaleCents))), new("stock", "Existencia inicial", N(p.Stock), ReadOnly: product is not null), new("minimum", "Existencia mínima", N(p.Minimum)), new("tax", "IVA %", N(p.TaxBasisPoints / 100m)), new("pack", "Presentación de compra", p.PackName), new("factor", "Unidades base por presentación", N(Amount.Quantity(p.PackUnits)))], async f =>
        {
            await catalog.Save(new Product { Id = p.Id, Sku = f["sku"], Barcode = f["barcode"], Name = f["name"], Category = f["category"], Brand = f["brand"], Size = f["size"], Unit = Enum.Parse<Unit>(f["unit"]), CostCents = string.IsNullOrWhiteSpace(f["cost"]) ? null : Amount.Cents(f.Number("cost")), PriceCents = Amount.Cents(f.Number("price")), WholesaleCents = Amount.Cents(f.Number("wholesale")), StockUnits = Amount.Units(f.Number("stock")), MinimumUnits = Amount.Units(f.Number("minimum")), TaxBasisPoints = checked((int)(f.Number("tax") * 100)), PackName = f["pack"], PackUnits = Amount.Units(f.Number("factor")), Active = p.Active });
        });
        if (saved) { Ui.Notify("Producto guardado."); await Refresh(); }
    }
    [RelayCommand] private async Task Archive()
    {
        if (Selected is null) return; var p = Selected;
        if (await dialogs.Confirm(p.Active ? "Archivar producto" : "Reactivar producto", $"{p.Name}. Su historial se conservará.", async () => { p.Active = !p.Active; await catalog.Save(p); })) await Refresh();
    }
    [RelayCommand] private async Task Adjust()
    {
        if (Selected is null) { Ui.Notify("Selecciona el producto que deseas ajustar."); return; } var p = Selected;
        if (await dialogs.Form("Ajustar existencias", $"{p.Name} · disponibles: {p.StockLabel}. Usa un número negativo para descontar.", [new("qty", "Cantidad a agregar o descontar", "0"), new("reason", "Motivo del ajuste", Multiline: true)], async f => { await catalog.Adjust(p.Id, f.Number("qty"), f["reason"]); }, "Registrar ajuste")) await Refresh();
    }
    [RelayCommand] private Task Movements() => Run(async () =>
    {
        if (Selected is null) throw new RuleException("Selecciona un producto para consultar sus movimientos."); var rows = await catalog.Movements(Selected.Id);
        dialogs.Table(new("Movimientos · " + Selected.Name, ["Fecha", "Origen", "Referencia", "Cantidad", "Existencia", "Motivo"], rows.Select(x => new[] { x.CreatedAt.ToString("dd/MM/yy HH:mm"), x.Kind, x.Reference, Amount.Quantity(x.DeltaUnits).ToString("+0.####;-0.####"), Amount.Quantity(x.BalanceUnits).ToString("0.####"), x.Reason }).ToList()));
    });
    [RelayCommand] private Task Export() => Run(async () => { var path = dialogs.SavePath("Productos", ".xlsx"); if (path is not null) { await import.Export(path); Ui.Notify("Catálogo exportado a Excel."); } });
    [RelayCommand] private Task Template() => Run(async () => { var path = dialogs.SavePath("Plantilla de productos", ".xlsx"); if (path is not null) await import.Template(path); });
    [RelayCommand] private Task Import() => Run(async () =>
    {
        var path = dialogs.OpenPath("Libro de Excel|*.xlsx"); if (path is null) return; var preview = await import.Preview(path);
        dialogs.Table(new("Vista previa de importación", ["Fila", "Clave", "Producto", "Resultado"], preview.Rows.Select(x => new[] { x.Row.ToString(), x.Product?.Sku ?? "", x.Product?.Name ?? "", x.Error.Length > 0 ? x.Error : "Nuevo producto" }).ToList()));
        if (!preview.Valid)
        {
            var report = dialogs.SavePath("Errores de importación", ".xlsx"); if (report is not null) await documents.Excel(report, new("Errores", ["Fila", "Error"], preview.Rows.Where(x => x.Error.Length > 0).Select(x => new[] { x.Row.ToString(), x.Error }).ToList()));
            Ui.Notify("Corrige las filas indicadas y vuelve a importar. No se guardó ningún producto.", true); return;
        }
        if (await dialogs.Confirm("Importar productos", $"Se agregarán {preview.Rows.Count} productos nuevos. No se modificarán productos existentes.", async () => { await import.Commit(preview); })) { var result = await catalog.Search(Search, page, ShowArchived); Items.Clear(); foreach (var p in result.Items) Items.Add(p); Ui.Notify("Importación completa."); }
    });
}
