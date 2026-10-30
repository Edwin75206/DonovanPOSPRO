using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Windows;
using DonovanPOS.Application;
using DonovanPOS.Infrastructure;
using DonovanPOS.Infrastructure.Persistence;
using DonovanPOS.Presentation.Common;
using DonovanPOS.Presentation.ViewModels;
using Microsoft.Extensions.DependencyInjection;
namespace DonovanPOS.Presentation;

public partial class App : System.Windows.Application
{
    private ServiceProvider? services;
    private Mutex? instanceMutex;
    private System.Windows.Threading.DispatcherTimer? backupTimer;
    private bool backingUp;
    private string backupError = "";
    protected override async void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e); CultureInfo.DefaultThreadCurrentCulture = Mx.Culture; CultureInfo.DefaultThreadCurrentUICulture = Mx.Culture;
        FrameworkElement.LanguageProperty.OverrideMetadata(typeof(FrameworkElement), new FrameworkPropertyMetadata(System.Windows.Markup.XmlLanguage.GetLanguage("es-MX")));
        var dataDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "DonovanSystems", "DonovanPOS", "data");
        var customIndex = Array.IndexOf(e.Args, "--data-dir"); if (customIndex >= 0 && customIndex + 1 < e.Args.Length) dataDirectory = Path.GetFullPath(e.Args[customIndex + 1]);
        var demo = e.Args.Contains("--demo");
#if DEBUG
        var qa = e.Args.Contains("--qa");
        demo |= qa;
#endif
        if (demo) dataDirectory = Path.Combine(Path.GetTempPath(), "DonovanPOS-Demo", Guid.NewGuid().ToString("N"));
        var mutexName = "Local\\DonovanPOS-" + Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(Path.GetFullPath(dataDirectory).ToUpperInvariant())))[..24];
        instanceMutex = new Mutex(true, mutexName, out var first);
        if (!first) { MessageBox.Show("Donovan POS ya está abierto para esta instalación.", "Donovan POS Pro"); Shutdown(); return; }
        var collection = new ServiceCollection().AddDonovan(dataDirectory); collection.AddSingleton<UiState>(); collection.AddSingleton<Dialogs>();
        if (demo) collection.AddSingleton<IClock>(new Demonstration.DemoClock());
        services = collection.BuildServiceProvider(); var ui = Get<UiState>();
        DispatcherUnhandledException += (_, args) => { ui.Notify(ui.Error(args.Exception), true); args.Handled = true; };
        try
        {
            await Get<Database>().Initialize();
            var vm = new MainVm(Get<AuthService>(), Get<UserSession>(), Get<SettingsService>(), Get<Dialogs>(), ui, BuildPages);
            var window = new MainWindow(vm); if (demo) { vm.IsDemo = true; window.Title += " · DEMOSTRACIÓN"; } MainWindow = window; window.Show();
            backupTimer = new() { Interval = TimeSpan.FromMinutes(5) }; backupTimer.Tick += async (_, _) => await AutomaticBackup(); backupTimer.Start();
            vm.PropertyChanged += async (_, change) => { if (change.PropertyName == nameof(MainVm.LoggedIn) && vm.LoggedIn) await AutomaticBackup(); };
            if (demo) { await Demonstration.DemoData.Prepare(services); await vm.SignedIn(); } else await vm.Initialize();
#if DEBUG
            var pageIndex = Array.IndexOf(e.Args, "--qa-page"); if (qa && pageIndex >= 0 && pageIndex + 1 < e.Args.Length) await vm.NavigateTo(e.Args[pageIndex + 1]);
            var auditIndex = Array.IndexOf(e.Args, "--qa-audit"); if (qa && auditIndex >= 0 && auditIndex + 1 < e.Args.Length) await QualityAssurance.VisualAudit.Run(window, vm, services, e.Args[auditIndex + 1]);
#endif
        }
        catch (Exception ex)
        {
            ui.Error(ex); MessageBox.Show("No fue posible iniciar Donovan POS. Revisa que la carpeta de datos sea accesible. Se registró el detalle técnico en la carpeta de logs.", "Donovan POS Pro", MessageBoxButton.OK, MessageBoxImage.Error);
#if DEBUG
            File.WriteAllText(Path.Combine(Path.GetTempPath(), "DonovanPOS-startup-error.txt"), ex.ToString());
#endif
            Shutdown(1);
        }
    }
    private T Get<T>() where T : notnull => services!.GetRequiredService<T>();
    private Dictionary<string, PageVm> BuildPages(Action<string> navigate, Func<Task> signedOut)
    {
        var dialogs = Get<Dialogs>(); var ui = Get<UiState>(); var settings = Get<SettingsService>(); var documents = Get<IDocumentService>(); var clock = Get<IClock>(); var session = Get<UserSession>();
        var pos = new PosVm(Get<CatalogService>(), Get<SalesService>(), Get<QuotesService>(), Get<CustomersService>(), settings, documents, dialogs, session, clock, Get<HeldSalesService>(), ui);
        return new()
        {
            ["dashboard"] = new DashboardVm(Get<IReportingService>(), clock, ui), ["pos"] = pos,
            ["products"] = new CatalogVm(Get<CatalogService>(), Get<ImportService>(), settings, documents, dialogs, ui, false),
            ["inventory"] = new InventoryVm(Get<CatalogService>(), Get<ImportService>(), settings, documents, dialogs, ui),
            ["customers"] = new CustomersVm(Get<CustomersService>(), Get<SalesService>(), dialogs, ui),
            ["quotes"] = new QuotesVm(Get<QuotesService>(), settings, documents, dialogs, ui, async (id, edit) => { if (id is int qid) await pos.LoadQuote(await Get<QuotesService>().Detail(qid), edit); navigate("pos"); }),
            ["purchases"] = new PurchasesVm(Get<PurchasesService>(), Get<CatalogService>(), dialogs, clock, ui),
            ["reports"] = new ReportsVm(Get<IReportingService>(), documents, settings, dialogs, clock, ui), ["cash"] = new CashVm(Get<CashService>(), dialogs, ui),
            ["users"] = new UsersVm(Get<AuthService>(), dialogs, ui), ["settings"] = new SettingsVm(settings, Get<IBackupService>(), documents, dialogs, ui, signedOut),
            ["history"] = new HistoryVm(Get<SalesService>(), Get<ReturnService>(), settings, documents, session, clock, dialogs, ui)
        };
    }
    private async Task AutomaticBackup()
    {
        if (backingUp || services is null) return; backingUp = true;
        try { await Get<IBackupService>().BackupIfDue(); backupError = ""; }
        catch (Exception ex) { var error = Get<UiState>().Error(ex); if (error != backupError) Get<UiState>().Notify("No se completó el respaldo automático. " + error, true); backupError = error; }
        finally { backingUp = false; }
    }
    protected override void OnExit(ExitEventArgs e) { backupTimer?.Stop(); services?.Dispose(); instanceMutex?.Dispose(); base.OnExit(e); }
}


