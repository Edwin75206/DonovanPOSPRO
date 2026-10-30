using System.IO;
using System.Printing;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class SettingsVm(SettingsService settings, IBackupService backups, IDocumentService documents, Dialogs dialogs, UiState ui, Func<Task> afterRestore) : PageVm(ui)
{
    public override string Title => "Configuración";
    [ObservableProperty] private StoreSettings config = new();
    [ObservableProperty] private string taxPercent = "16";
    [ObservableProperty] private string lastBackup = "Sin respaldos registrados";
    public int[] BackupIntervals { get; } = [1, 4, 8, 24, 168];
    public string DataPath => backups.DataDirectory;
    public string[] Printers { get; private set; } = [""];
    public int[] TicketWidths { get; } = [58, 80];
    public override Task Load() => Run(async () =>
    {
        Config = await settings.Get(); TaxPercent = (Config.DefaultTaxBasisPoints / 100m).ToString("0.##", Mx.Culture);
        await UpdateBackupStatus();
        try { using var server = new LocalPrintServer(); Printers = new[] { "" }.Concat(server.GetPrintQueues().Select(x => x.Name)).ToArray(); OnPropertyChanged(nameof(Printers)); } catch (PrintSystemException) { Ui.Notify("Windows no devolvió impresoras disponibles. Puedes seguir generando PDF."); }
    });
    [RelayCommand] private Task Save() => Run(async () => { Config.DefaultTaxBasisPoints = checked((int)(Mx.Number(TaxPercent) * 100)); await settings.Save(Config); ThemeManager.ReducedMotion = Config.ReducedMotion; ThemeManager.Apply(Config.DarkMode); }, "Configuración guardada.");
    [RelayCommand] private Task Logo() => Run(async () =>
    {
        var file = dialogs.OpenPath("Logotipo PNG o JPEG|*.png;*.jpg;*.jpeg"); if (file is null) return;
        Config.LogoData = await documents.ReadLogo(file); Config.LogoPath = Path.GetFileName(file); OnPropertyChanged(nameof(Config)); Ui.Notify("Logotipo seleccionado. Guarda la configuración; quedará incluido en los respaldos.");
    });
    [RelayCommand] private Task Backup() => Run(async () => { var folder = string.IsNullOrWhiteSpace(Config.BackupDirectory) ? dialogs.Folder() : Config.BackupDirectory; if (folder is null) return; var file = await backups.Backup(folder); await UpdateBackupStatus(); Ui.Notify("Respaldo verificado: " + file); });
    private async Task UpdateBackupStatus() { var latest = (await backups.History()).FirstOrDefault(x => x.Successful); LastBackup = latest is null ? "Sin respaldos registrados" : $"Última copia verificada: {latest.CreatedAt:dd/MM/yyyy HH:mm} · {latest.Kind}\n{latest.FilePath}"; }
    [RelayCommand] private Task BackupHistory() => Run(async () => { var rows = await backups.History(); dialogs.Table(new("Historial de respaldos", ["Fecha", "Tipo", "Resultado", "Archivo", "Tamaño", "SHA-256"], rows.Select(x => new[] { x.CreatedAt.ToString("dd/MM/yyyy HH:mm"), x.Kind, x.Result, x.FilePath, x.Successful ? (x.FileBytes / 1024m).ToString("N0") + " KB" : "—", x.Sha256 }).ToList())); });
    [RelayCommand] private void BackupFolder() { var folder = dialogs.Folder(); if (folder is not null) { Config.BackupDirectory = folder; OnPropertyChanged(nameof(Config)); } }
    [RelayCommand] private async Task Restore()
    {
        var file = dialogs.OpenPath("Respaldo Donovan POS|*.db"); if (file is null) return;
        if (await dialogs.Confirm("Restaurar respaldo", "Se reemplazarán los datos actuales por el respaldo seleccionado. Se conservará una copia de seguridad de los datos actuales y tendrás que iniciar sesión otra vez.", () => backups.Restore(file))) await afterRestore();
    }
    [RelayCommand] private Task Audit() => Run(async () => { var events = await settings.Audit(); dialogs.Table(new("Bitácora de actividad", ["Fecha", "Usuario ID", "Acción", "Referencia", "Detalle"], events.Select(x => new[] { x.CreatedAt.ToString("dd/MM/yyyy HH:mm"), x.UserId?.ToString() ?? "Instalación", x.Action, x.Reference, x.Detail }).ToList())); });
    [RelayCommand] private void About() => dialogs.Table(new("Acerca de Donovan POS Pro", ["Producto", "Información"], [["Fabricante", "Donovan Systems"], ["Responsable", "Ing. Edwin Donovan Castañeda Calderón"], ["Versión", "1.0.0 · Windows x64"], ["Moneda", "MXN · Español de México"], ["Licencias", "Ver carpeta licenses junto al programa"], ["Datos", DataPath]]));
}
