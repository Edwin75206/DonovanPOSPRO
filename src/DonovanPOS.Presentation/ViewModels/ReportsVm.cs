using System.Data;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class ReportsVm(IReportingService reports, IDocumentService documents, SettingsService settings, Dialogs dialogs, IClock clock, UiState ui) : PageVm(ui)
{
    public override string Title => "Reportes";
    public string[] Kinds { get; } = ["Ventas", "Por producto", "Por categoría", "Por usuario", "Métodos de pago", "Inventario", "Costos y ganancias", "Compras", "Cuentas por cobrar", "Devoluciones"];
    [ObservableProperty] private string selectedKind = "Ventas";
    [ObservableProperty] private DateTime from = clock.Now.Date.AddDays(-6);
    [ObservableProperty] private DateTime to = clock.Now.Date;
    [ObservableProperty] private DataView? rows;
    [ObservableProperty] private string caption = "";
    private TableData? table;
    public override Task Load() => Refresh();
    [RelayCommand] private Task Refresh() => Run(async () =>
    {
        table = await reports.Report(SelectedKind, From, To); var dt = new DataTable(); foreach (var header in table.Headers) dt.Columns.Add(header); foreach (var row in table.Rows) dt.Rows.Add(row.Cast<object>().ToArray()); Rows = dt.DefaultView; Caption = table.Title + $" · {table.Rows.Count:N0} registros";
    });
    [RelayCommand] private Task Excel() => Run(async () => { if (table is null) return; var path = dialogs.SavePath(SelectedKind, ".xlsx"); if (path is not null) { await documents.Excel(path, table); Ui.Notify("Reporte exportado a Excel."); } });
    [RelayCommand] private Task Pdf() => Run(async () => { if (table is null) return; var path = dialogs.SavePath(SelectedKind, ".pdf"); if (path is not null) { await documents.ReportPdf(path, await settings.Get(), table); Mx.OpenFile(path); } });
}
