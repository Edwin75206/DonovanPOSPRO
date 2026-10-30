#if DEBUG
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using DonovanPOS.Application;
using DonovanPOS.Presentation.Common;
using DonovanPOS.Presentation.ViewModels;
using Microsoft.Extensions.DependencyInjection;
namespace DonovanPOS.Presentation.QualityAssurance;

/// <summary>Own-application layout regression harness. These images are WPF renders, not OS screenshots.</summary>
internal static class VisualAudit
{
    public static async Task Run(MainWindow window, MainVm vm, ServiceProvider services, string folder)
    {
        Directory.CreateDirectory(folder); var errors = new List<string>(); var renders = new List<object>();
        var ui = services.GetRequiredService<UiState>(); var listener = new BindingErrors(); PresentationTraceSources.DataBindingSource.Listeners.Add(listener); PresentationTraceSources.DataBindingSource.Switch.Level = SourceLevels.Error;
        System.Windows.Application.Current.DispatcherUnhandledException += (_, e) => { errors.Add(e.Exception.ToString()); e.Handled = true; };
        foreach (var (width, height, tag) in new[] { (1366, 768, "1366x768-100"), (1920, 1080, "1920x1080-100"), (1093, 614, "1366x768-125-simulated"), (911, 512, "1366x768-150-simulated"), (1536, 864, "1920x1080-125-simulated"), (1280, 720, "1920x1080-150-simulated") })
        {
            window.Width = width; window.Height = height;
            foreach (var dark in new[] { false, true })
            {
                ThemeManager.Apply(dark);
                foreach (var page in new[] { "dashboard", "pos", "inventory" })
                {
                    ui.Notification = ""; await vm.NavigateTo(page); await Task.Delay(250); await window.Dispatcher.InvokeAsync(window.UpdateLayout, DispatcherPriority.ContextIdle);
                    if (ui.IsError && ui.Notification.Length > 0) errors.Add(page + ": " + ui.Notification);
                    var filename = $"{page}-{tag}-{(dark ? "dark" : "light")}.png"; Capture(window, Path.Combine(folder, filename)); renders.Add(new { page, width, height, dark, file = filename });
                }
            }
        }
        window.Width = 1366; window.Height = 768; ThemeManager.Apply(false);
        foreach (var page in new[] { "products", "customers", "quotes", "purchases", "reports", "cash", "users", "settings", "history" })
        {
            ui.Notification = ""; await vm.NavigateTo(page); await Task.Delay(200); window.UpdateLayout(); Capture(window, Path.Combine(folder, page + "-1366x768-light.png")); if (ui.IsError && ui.Notification.Length > 0) errors.Add(page + ": " + ui.Notification);
        }
        var docs = services.GetRequiredService<IDocumentService>(); var config = await services.GetRequiredService<SettingsService>().Get(); var sale = (await services.GetRequiredService<SalesService>().History(DateTime.Today.AddDays(-30), DateTime.Today)).First(); var receipt = PosVm.Receipt(await services.GetRequiredService<SalesService>().Detail(sale.Id));
        await docs.Pdf(Path.Combine(folder, "ticket-58.pdf"), config, receipt, 58); await docs.Pdf(Path.Combine(folder, "ticket-80.pdf"), config, receipt, 80);
        var quote = await services.GetRequiredService<QuotesService>().Detail((await services.GetRequiredService<QuotesService>().List()).First().Id);
        var proposal = new BusinessDocument("Cotización", quote.Quote.Folio, quote.Quote.CreatedAt, quote.Quote.CustomerName, quote.Lines.Select(x => new DocumentLine(x.Name, DonovanPOS.Domain.Amount.Quantity(x.QuantityUnits) + " " + x.Unit, DonovanPOS.Domain.Amount.Pesos(x.PriceCents), DonovanPOS.Domain.Amount.Pesos(x.TotalCents))).ToList(), DonovanPOS.Domain.Amount.Pesos(quote.Quote.TotalCents), DonovanPOS.Domain.Amount.Pesos(quote.Quote.TaxCents), "Válida hasta " + quote.Quote.ValidUntil.ToString("dd/MM/yyyy") + "\n" + quote.Quote.Notes);
        await docs.Pdf(Path.Combine(folder, "cotizacion.pdf"), config, proposal); await docs.Pdf(Path.Combine(folder, "cotizacion-multipagina.pdf"), config, proposal with { Lines = Enumerable.Range(1, 60).Select(i => new DocumentLine("Producto de prueba con una descripción larga y medidas 1/4 x 2 pulgadas " + i, "2 Pieza", 116, 232)).ToList(), Total = 13920, Tax = 1920 });
        await docs.Labels(Path.Combine(folder, "etiquetas.pdf"), config, [new("7501234567890", "Tornillo galvanizado 1/4 x 2 pulgadas", 1250, 17)]);
        await docs.ReportPdf(Path.Combine(folder, "reporte-ventas.pdf"), config, await services.GetRequiredService<IReportingService>().Report("Ventas", DateTime.Today.AddDays(-30), DateTime.Today));
        var login = new LoginVm(services.GetRequiredService<AuthService>(), services.GetRequiredService<Dialogs>(), ui, () => Task.CompletedTask) { Setup = true };
        window.Content = new Views.WelcomeView { DataContext = login };
        for (var step = 0; step < 4; step++) { login.SetupStep = step; await Task.Delay(100); window.UpdateLayout(); Capture(window, Path.Combine(folder, "configuracion-inicial-" + (step + 1) + ".png")); }
        if (listener.Errors.Length > 0) errors.Add(listener.Errors.ToString());
        await File.WriteAllTextAsync(Path.Combine(folder, "visual-audit.json"), JsonSerializer.Serialize(new { generated = DateTime.Now, renderType = "WPF RenderTargetBitmap; scaled cases simulate logical viewport, not a Windows DPI change", errors, renders }, new JsonSerializerOptions { WriteIndented = true }));
        System.Windows.Application.Current.Shutdown(errors.Count == 0 ? 0 : 2);
    }
    private static void Capture(Window window, string path)
    {
        var visual = (FrameworkElement)window.Content; var image = new RenderTargetBitmap((int)Math.Ceiling(visual.ActualWidth), (int)Math.Ceiling(visual.ActualHeight), 96, 96, PixelFormats.Pbgra32); image.Render(visual);
        var encoder = new PngBitmapEncoder(); encoder.Frames.Add(BitmapFrame.Create(image)); using var file = File.Create(path); encoder.Save(file);
    }
    private sealed class BindingErrors : TraceListener
    {
        public StringBuilder Errors { get; } = new();
        public override void Write(string? message) => Errors.Append(message);
        public override void WriteLine(string? message) => Errors.AppendLine(message);
    }
}
#endif
