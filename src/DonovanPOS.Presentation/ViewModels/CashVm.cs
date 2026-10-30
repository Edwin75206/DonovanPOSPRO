using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class CashVm(CashService cash, Dialogs dialogs, UiState ui) : PageVm(ui)
{
    public override string Title => "Caja";
    [ObservableProperty] private CashOverview? overview;
    public ObservableCollection<RecordRow> Movements { get; } = [];
    public ObservableCollection<RecordRow> History { get; } = [];
    public string Status => Overview?.Session is null ? "Caja cerrada" : "Caja abierta · " + Overview.Session.Folio;
    public string Expected => Mx.Money(Overview?.ExpectedCents ?? 0);
    public string Opening => Mx.Money(Overview?.Session?.OpeningCents ?? 0);
    public string Incomes => Mx.Money(Overview?.Movements.Where(x => x.AmountCents > 0).Sum(x => x.AmountCents) ?? 0);
    public string Expenses => Mx.Money(-(Overview?.Movements.Where(x => x.AmountCents < 0).Sum(x => x.AmountCents) ?? 0));
    public bool Opened => Overview?.Session is not null;
    public override Task Load() => Refresh();
    [RelayCommand] private Task Refresh() => Run(async () =>
    {
        Overview = await cash.Overview(); Movements.Clear(); foreach (var x in Overview.Movements) Movements.Add(new(x.Id, x.CreatedAt.ToString("HH:mm"), x.Kind, x.Reference, Mx.Money(x.AmountCents), x.Reason));
        History.Clear(); foreach (var x in Overview.History) History.Add(new(x.Id, x.Folio, x.ClosedAt?.ToString("dd/MM/yyyy HH:mm") ?? "", Mx.Money(x.ExpectedCents ?? 0), Mx.Money(x.CountedCents ?? 0), Mx.Money((x.CountedCents ?? 0) - (x.ExpectedCents ?? 0))));
        foreach (var p in new[] { nameof(Status), nameof(Expected), nameof(Opening), nameof(Incomes), nameof(Expenses), nameof(Opened) }) OnPropertyChanged(p);
    });
    [RelayCommand] private async Task Open() { if (await dialogs.Form("Abrir caja", "Cuenta el efectivo que tendrás como fondo inicial.", [new("amount", "Fondo inicial MXN", "0")], async f => { await cash.Open(f.Number("amount")); }, "Abrir caja")) await Refresh(); }
    [RelayCommand] private async Task Movement() { if (await dialogs.Form("Registrar ingreso o egreso", "Usa un importe positivo para ingreso y negativo para egreso. No registres aquí ventas ni abonos: sus movimientos son automáticos.", [new("amount", "Importe MXN", "0"), new("reason", "Motivo", Multiline: true)], async f => { await cash.Movement(f.Number("amount"), f["reason"]); }, "Registrar movimiento")) await Refresh(); }
    [RelayCommand] private async Task Close()
    {
        if (await dialogs.Form("Cerrar caja", "Efectivo esperado: " + Expected + ". Cuenta únicamente efectivo; tarjeta, transferencia y crédito se consultan en reportes.", [new("amount", "Efectivo contado MXN"), new("notes", "Observaciones", Multiline: true)], async f => { var result = await cash.Close(f.Number("amount"), f["notes"]); Ui.Notify($"Cierre {result.Folio} guardado. Diferencia: {Mx.Money((result.CountedCents ?? 0) - (result.ExpectedCents ?? 0))}"); }, "Confirmar cierre", f => "Diferencia: " + Mx.Money(Amount.Cents(f.Number("amount")) - (Overview?.ExpectedCents ?? 0)))) await Refresh();
    }
}
