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
    protected override async void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e); CultureInfo.DefaultThreadCurrentCulture = Mx.Culture; CultureInfo.DefaultThreadCurrentUICulture = Mx.Culture;
        FrameworkElement.LanguageProperty.OverrideMetadata(typeof(FrameworkElement), new FrameworkPropertyMetadata(System.Windows.Markup.XmlLanguage.GetLanguage("es-MX")));
        var dataDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "DonovanSystems", "DonovanPOS", "data");
        var customIndex = Array.IndexOf(e.Args, "--data-dir"); if (customIndex >= 0 && customIndex + 1 < e.Args.Length) dataDirectory = Path.GetFullPath(e.Args[customIndex + 1]);
#if DEBUG
        var qa = e.Args.Contains("--qa");
        if (qa) dataDirectory = Path.Combine(Path.GetTempPath(), "DonovanPOS-QA", Guid.NewGuid().ToString("N"));
#endif
        var mutexName = "Local\\DonovanPOS-" + Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(Path.GetFullPath(dataDirectory).ToUpperInvariant())))[..24];
        instanceMutex = new Mutex(true, mutexName, out var first);
        if (!first) { MessageBox.Show("Donovan POS ya está abierto para esta instalación.", "Donovan POS Pro"); Shutdown(); return; }
        var collection = new ServiceCollection().AddDonovan(dataDirectory); collection.AddSingleton<UiState>(); collection.AddSingleton<Dialogs>();
#if DEBUG
        if (qa) collection.AddSingleton<IClock>(new QualityAssurance.QaClock());
#endif
        services = collection.BuildServiceProvider(); var ui = Get<UiState>();
        DispatcherUnhandledException += (_, args) => { ui.Notify(ui.Error(args.Exception), true); args.Handled = true; };
        try
        {
            await Get<Database>().Initialize();
            var vm = new MainVm(Get<AuthService>(), Get<UserSession>(), Get<SettingsService>(), Get<Dialogs>(), ui, BuildPages);
            var window = new MainWindow(vm); MainWindow = window; window.Show();
#if DEBUG
            if (qa) { await QualityAssurance.QaData.Prepare(services); await vm.SignedIn(); } else
#endif
            await vm.Initialize();
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
        var pos = new PosVm(Get<CatalogService>(), Get<SalesService>(), Get<QuotesService>(), Get<CustomersService>(), settings, documents, dialogs, session, clock, ui);
        return new()
        {
            ["dashboard"] = new DashboardVm(Get<IReportingService>(), clock, ui), ["pos"] = pos,
            ["products"] = new CatalogVm(Get<CatalogService>(), Get<ImportService>(), settings, documents, dialogs, ui, false),
            ["inventory"] = new CatalogVm(Get<CatalogService>(), Get<ImportService>(), settings, documents, dialogs, ui, true),
            ["customers"] = new CustomersVm(Get<CustomersService>(), Get<SalesService>(), dialogs, ui),
            ["quotes"] = new QuotesVm(Get<QuotesService>(), settings, documents, dialogs, ui, async (id, edit) => { if (id is int qid) await pos.LoadQuote(await Get<QuotesService>().Detail(qid), edit); navigate("pos"); }),
            ["purchases"] = new PurchasesVm(Get<PurchasesService>(), Get<CatalogService>(), dialogs, clock, ui),
            ["reports"] = new ReportsVm(Get<IReportingService>(), documents, settings, dialogs, clock, ui), ["cash"] = new CashVm(Get<CashService>(), dialogs, ui),
            ["users"] = new UsersVm(Get<AuthService>(), dialogs, ui), ["settings"] = new SettingsVm(settings, Get<IBackupService>(), dialogs, ui, signedOut),
            ["history"] = new HistoryVm(Get<SalesService>(), Get<ReturnService>(), settings, documents, session, clock, dialogs, ui)
        };
    }
    protected override void OnExit(ExitEventArgs e) { services?.Dispose(); instanceMutex?.Dispose(); base.OnExit(e); }
}
