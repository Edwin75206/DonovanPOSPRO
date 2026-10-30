using System.IO;
using System.Printing;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class SettingsVm(SettingsService settings, IBackupService backups, Dialogs dialogs, UiState ui, Func<Task> afterRestore) : PageVm(ui)
{
    public override string Title => "Configuración";
    [ObservableProperty] private StoreSettings config = new();
    [ObservableProperty] private string taxPercent = "16";
    public string DataPath => backups.DataDirectory;
    public string[] Printers { get; private set; } = [""];
    public int[] TicketWidths { get; } = [58, 80];
    public override Task Load() => Run(async () =>
    {
        Config = await settings.Get(); TaxPercent = (Config.DefaultTaxBasisPoints / 100m).ToString("0.##", Mx.Culture);
        try { using var server = new LocalPrintServer(); Printers = new[] { "" }.Concat(server.GetPrintQueues().Select(x => x.Name)).ToArray(); OnPropertyChanged(nameof(Printers)); } catch (PrintSystemException) { Ui.Notify("Windows no devolvió impresoras disponibles. Puedes seguir generando PDF."); }
    });
    [RelayCommand] private Task Save() => Run(async () => { Config.DefaultTaxBasisPoints = checked((int)(Mx.Number(TaxPercent) * 100)); await settings.Save(Config); ThemeManager.ReducedMotion = Config.ReducedMotion; ThemeManager.Apply(Config.DarkMode); }, "Configuración guardada.");
    [RelayCommand] private Task Logo() => Run(() =>
    {
        var file = dialogs.OpenPath("Logotipo PNG o JPEG|*.png;*.jpg;*.jpeg"); if (file is null) return Task.CompletedTask;
        Amount.Require(new FileInfo(file).Length < 2 * 1024 * 1024, "El logotipo debe ser menor de 2 MB.");
        var folder = Path.Combine(backups.DataDirectory, "branding"); Directory.CreateDirectory(folder); var destination = Path.Combine(folder, Guid.NewGuid().ToString("N") + Path.GetExtension(file)); File.Copy(file, destination);
        Config.LogoPath = destination; OnPropertyChanged(nameof(Config)); Ui.Notify("Logotipo seleccionado. Guarda la configuración."); return Task.CompletedTask;
    });
    [RelayCommand] private Task Backup() => Run(async () => { var folder = string.IsNullOrWhiteSpace(Config.BackupDirectory) ? dialogs.Folder() : Config.BackupDirectory; if (folder is null) return; var file = await backups.Backup(folder); Ui.Notify("Respaldo creado: " + file); });
    [RelayCommand] private void BackupFolder() { var folder = dialogs.Folder(); if (folder is not null) { Config.BackupDirectory = folder; OnPropertyChanged(nameof(Config)); } }
    [RelayCommand] private async Task Restore()
    {
        var file = dialogs.OpenPath("Respaldo Donovan POS|*.db"); if (file is null) return;
        if (await dialogs.Confirm("Restaurar respaldo", "Se reemplazarán los datos actuales por el respaldo seleccionado. Se conservará una copia de seguridad de los datos actuales y tendrás que iniciar sesión otra vez.", () => backups.Restore(file))) await afterRestore();
    }
    [RelayCommand] private Task Audit() => Run(async () => { var events = await settings.Audit(); dialogs.Table(new("Bitácora de actividad", ["Fecha", "Usuario ID", "Acción", "Referencia", "Detalle"], events.Select(x => new[] { x.CreatedAt.ToString("dd/MM/yyyy HH:mm"), x.UserId?.ToString() ?? "Instalación", x.Action, x.Reference, x.Detail }).ToList())); });
    [RelayCommand] private void About() => dialogs.Table(new("Acerca de Donovan POS Pro", ["Producto", "Información"], [["Fabricante", "Donovan Systems"], ["Responsable", "Ing. Edwin Donovan Castañeda Calderón"], ["Versión", "1.0.0 · Windows x64"], ["Moneda", "MXN · Español de México"], ["Licencias", "Ver carpeta licenses junto al programa"], ["Datos", DataPath]]));
}
