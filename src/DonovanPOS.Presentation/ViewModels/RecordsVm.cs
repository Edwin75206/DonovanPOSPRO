using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed record RecordRow(int Id, string A, string B, string C, string D, string E = "", object? Source = null);
public sealed record PageAction(string Label, IAsyncRelayCommand Command, bool Primary = false);
public abstract partial class RecordsVm(UiState ui) : PageVm(ui)
{
    public ObservableCollection<RecordRow> Rows { get; } = [];
    public ObservableCollection<PageAction> Actions { get; } = [];
    public ObservableCollection<PageAction> RowActions { get; } = [];
    [ObservableProperty] private RecordRow? selected;
    [ObservableProperty] private string summary = "";
    [ObservableProperty] private string search = "";
    public abstract string Subtitle { get; }
    public abstract string[] Headers { get; }
    protected void SetRows(IEnumerable<RecordRow> rows) { Rows.Clear(); foreach (var row in rows) Rows.Add(row); }
    public override Task Load() => RefreshCommand.ExecuteAsync(null);
    [RelayCommand] private Task Refresh() => Run(Reload);
    protected abstract Task Reload();
    protected PageAction Action(string label, Func<Task> action, bool primary = false) => new(label, new AsyncRelayCommand(() => Run(action)), primary);
    protected int SelectedId() => Selected?.Id ?? throw new DonovanPOS.Domain.RuleException("Selecciona una fila para continuar.");
}
