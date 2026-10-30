using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed class HistoryVm : RecordsVm
{
    private readonly SalesService sales; private readonly ReturnService returns; private readonly SettingsService settings; private readonly IDocumentService documents; private readonly Dialogs dialogs; private readonly IClock clock;
    public HistoryVm(SalesService sales, ReturnService returns, SettingsService settings, IDocumentService documents, UserSession session, IClock clock, Dialogs dialogs, UiState ui) : base(ui)
    {
        this.sales = sales; this.returns = returns; this.settings = settings; this.documents = documents; this.dialogs = dialogs; this.clock = clock;
        Actions.Add(Action("Cambiar período", Dates)); RowActions.Add(Action("Consultar ticket", Detail, true)); RowActions.Add(Action("Ticket PDF", Pdf)); RowActions.Add(Action("Imprimir ticket", Print));
        if (session.Can(Permission.Return)) { RowActions.Add(Action("Devolver productos", () => Refund(false))); RowActions.Add(Action("Cancelar venta", () => Refund(true))); }
        from = clock.Now.Date.AddDays(-30); to = clock.Now.Date;
    }
    private DateTime from, to; private int page;
    public override string Title => "Historial de ventas";
    public override string Subtitle => "Tickets, devoluciones y cancelaciones con trazabilidad.";
    public override string[] Headers => ["Folio", "Fecha", "Cliente", "Total MXN", "Estado"];
    protected override async Task Reload()
    {
        var rows = await sales.History(from, to, page); Summary = $"{from:dd/MM/yyyy} — {to:dd/MM/yyyy} · hasta 100 ventas más recientes del período";
        SetRows(rows.Where(x => (x.Folio + x.CustomerName).Contains(Search, StringComparison.CurrentCultureIgnoreCase)).Select(x => new RecordRow(x.Id, x.Folio, x.CreatedAt.ToString("dd/MM/yyyy HH:mm"), x.CustomerName, Mx.Money(x.TotalCents), x.Cancelled ? "Cancelada / devuelta" : "Registrada", x)));
    }
    private async Task Dates()
    {
        if (await dialogs.Form("Consultar ventas", "Selecciona el período que deseas revisar.", [new("from", "Desde (dd/MM/aaaa)", from.ToString("dd/MM/yyyy")), new("to", "Hasta (dd/MM/aaaa)", to.ToString("dd/MM/yyyy")), new("page", "Página (100 ventas por página)", (page + 1).ToString())], f => { var a = DateTime.ParseExact(f["from"], "dd/MM/yyyy", Mx.Culture); var b = DateTime.ParseExact(f["to"], "dd/MM/yyyy", Mx.Culture); Amount.Require(a <= b && f.Integer("page") >= 1, "Revisa las fechas y la página."); from = a; to = b; page = f.Integer("page") - 1; return Task.CompletedTask; }, "Consultar")) await Reload();
    }
    private async Task Detail() { var d = await sales.Detail(SelectedId()); dialogs.Table(new(d.Sale.Folio + " · " + d.Sale.CustomerName, ["Producto", "Cantidad", "Precio", "Total", "Devuelto"], d.Lines.Select(x => new[] { x.Name, $"{Amount.Quantity(x.QuantityUnits):0.####} {x.Unit}", Mx.Money(x.PriceCents), Mx.Money(x.TotalCents), Amount.Quantity(x.ReturnedUnits).ToString("0.####") }).ToList())); }
    private async Task Pdf() { var d = await sales.Detail(SelectedId()); var path = dialogs.SavePath(d.Sale.Folio, ".pdf"); if (path is not null) { var s = await settings.Get(); await documents.Pdf(path, s, PosVm.Receipt(d), s.TicketWidthMm); Mx.OpenFile(path); } }
    private async Task Print() => dialogs.Print(await settings.Get(), PosVm.Receipt(await sales.Detail(SelectedId())));
    private async Task Refund(bool cancel)
    {
        var d = await sales.Detail(SelectedId()); var choices = d.Lines.Where(x => x.ReturnedUnits < x.QuantityUnits).Select(x => new Choice(x.Id.ToString(), x.Name + " · quedan " + Amount.Quantity(x.QuantityUnits - x.ReturnedUnits).ToString("0.####"))).ToArray();
        Amount.Require(choices.Length > 0, "La venta ya fue devuelta por completo.");
        List<Field> fields = [new("reason", "Motivo", Multiline: true), new("method", "Cómo entregarás el reembolso", "Efectivo", Dialogs.Options<PayMethod>().Where(x => x.Value != "Credito").ToArray())];
        if (!cancel) { fields.Add(new("line", "Producto", choices[0].Value, choices)); fields.Add(new("qty", "Cantidad a devolver", "1")); }
        if (await dialogs.Form(cancel ? "Cancelar venta " + d.Sale.Folio : "Devolver productos", "Se reintegrarán las existencias. Si hay deuda, primero se reducirá el saldo; el resto se registrará como reembolso por el método elegido.", fields, async f => { await returns.Return(d.Sale.Id, cancel ? [] : [new(f.Integer("line"), f.Number("qty"))], f["reason"], Enum.Parse<PayMethod>(f["method"]), cancel); }, "Confirmar " + (cancel ? "cancelación" : "devolución"))) await Reload();
    }
}
