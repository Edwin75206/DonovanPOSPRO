using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed class QuotesVm : RecordsVm
{
    private readonly QuotesService quotes; private readonly SettingsService settings; private readonly IDocumentService documents; private readonly Dialogs dialogs;
    public QuotesVm(QuotesService quotes, SettingsService settings, IDocumentService documents, Dialogs dialogs, UiState ui, Func<int?, bool, Task> open) : base(ui)
    {
        this.quotes = quotes; this.settings = settings; this.documents = documents; this.dialogs = dialogs;
        Actions.Add(Action("Nueva cotización", () => open(null, true), true));
        RowActions.Add(Action("Convertir en venta", () => open(SelectedId(), false), true));
        RowActions.Add(Action("Editar partidas", async () => { if (await dialogs.Confirm("Editar cotización", "Se abrirán sus partidas con los precios actuales del catálogo. Podrás ajustar cantidades y descuento antes de guardar.", () => Task.CompletedTask)) await open(SelectedId(), true); }));
        RowActions.Add(Action("Generar PDF", Pdf));
    }
    public override string Title => "Cotizaciones";
    public override string Subtitle => "Prepara una propuesta y conviértela en venta cuando tu cliente decida.";
    public override string[] Headers => ["Folio", "Cliente", "Vigencia", "Total MXN", "Estado"];
    protected override async Task Reload()
    {
        var rows = await quotes.List(); Summary = $"{rows.Count(x => x.SaleId is null)} cotizaciones sin convertir · el inventario se descuenta al vender";
        SetRows(rows.Where(x => (x.CustomerName + x.Folio).Contains(Search, StringComparison.CurrentCultureIgnoreCase)).Select(x => new RecordRow(x.Id, x.Folio, x.CustomerName, x.ValidUntil.ToString("dd/MM/yyyy"), Mx.Money(x.TotalCents), x.SaleId is not null ? "Convertida en venta" : x.ValidUntil.Date < DateTime.Today ? "Vencida" : "Vigente", x)));
    }
    private async Task Pdf()
    {
        var detail = await quotes.Detail(SelectedId()); var path = dialogs.SavePath(detail.Quote.Folio, ".pdf"); if (path is null) return;
        var q = detail.Quote; await documents.Pdf(path, await settings.Get(), new("Cotización", q.Folio, q.CreatedAt, q.CustomerName, detail.Lines.Select(x => new DocumentLine(x.Name, $"{Amount.Quantity(x.QuantityUnits):0.####} {x.Unit}", Amount.Pesos(x.PriceCents), Amount.Pesos(x.TotalCents))).ToList(), Amount.Pesos(q.TotalCents), Amount.Pesos(q.TaxCents), "Válida hasta " + q.ValidUntil.ToString("dd/MM/yyyy") + "\n" + q.Notes)); Mx.OpenFile(path);
    }
}
