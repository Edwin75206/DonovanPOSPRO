using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Data;
using System.Windows.Media.Animation;
using CommunityToolkit.Mvvm.ComponentModel;
using DonovanPOS.Application;
using DonovanPOS.Domain;
namespace DonovanPOS.Presentation.Common;

public sealed class InverseBooleanConverter : IValueConverter { public object Convert(object value, Type type, object p, CultureInfo c) => value is not true; public object ConvertBack(object value, Type type, object p, CultureInfo c) => value is not true; }
public sealed class MoneyConverter : IValueConverter { public object Convert(object value, Type type, object p, CultureInfo c) => value is long n ? Amount.Pesos(n).ToString("C2", Mx.Culture) : "—"; public object ConvertBack(object value, Type type, object p, CultureInfo c) => Binding.DoNothing; }
public static class Mx
{
    public static readonly CultureInfo Culture = CultureInfo.GetCultureInfo("es-MX");
    public static string Money(long cents) => Amount.Pesos(cents).ToString("C2", Culture);
    public static decimal Number(string value) { if (decimal.TryParse(value, NumberStyles.Number, Culture, out var n)) return n; throw new RuleException("Escribe un número válido; por ejemplo, 125.50."); }
    public static void OpenFile(string path) => Process.Start(new ProcessStartInfo(path) { UseShellExecute = true });
}
public sealed partial class UiState : ObservableObject
{
    [ObservableProperty] private string notification = "";
    [ObservableProperty] private bool isError;
    public void Notify(string text, bool error = false) { IsError = error; Notification = text; }
    public string Error(Exception ex)
    {
        if (ex is RuleException) return ex.Message;
        if (ex is FormatException or OverflowException) return "Revisa los números y las fechas. Usa el formato dd/MM/aaaa para fechas y un punto decimal para importes.";
        if (ex is IOException or UnauthorizedAccessException) return "No se pudo acceder al archivo. Cierra otras aplicaciones que lo estén usando y revisa la carpeta seleccionada.";
        var directory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "DonovanSystems", "DonovanPOS", "logs");
        Directory.CreateDirectory(directory);
        File.AppendAllText(Path.Combine(directory, DateTime.Today.ToString("yyyyMMdd") + ".jsonl"), JsonSerializer.Serialize(new { timestamp = DateTime.UtcNow, exception = ex.GetType().Name, stack = ex.StackTrace }) + Environment.NewLine);
        return "No se completó la operación. Tus cambios no confirmados no se guardaron. Intenta de nuevo o consulta soporte.";
    }
}
public abstract partial class PageVm(UiState ui) : ObservableObject
{
    protected UiState Ui { get; } = ui;
    [ObservableProperty] private bool busy;
    public abstract string Title { get; }
    public virtual Task Load() => Task.CompletedTask;
    protected async Task Run(Func<Task> action, string? success = null)
    {
        if (Busy) return; Busy = true;
        try { await action(); if (success is not null) Ui.Notify(success); }
        catch (Exception ex) { Ui.Notify(Ui.Error(ex), true); }
        finally { Busy = false; }
    }
}
public static class ThemeManager
{
    public static bool ReducedMotion { get; set; }
    public static bool Dark { get; private set; }
    public static void Apply(bool dark)
    {
        Dark = dark;
        System.Windows.Application.Current.Resources.MergedDictionaries[0] = new ResourceDictionary { Source = new Uri($"Themes/{(dark ? "Dark" : "Light")}.xaml", UriKind.Relative) };
    }
    public static void Enter(UIElement element)
    {
        if (ReducedMotion || !SystemParameters.ClientAreaAnimation) return;
        element.BeginAnimation(UIElement.OpacityProperty, new DoubleAnimation(.35, 1, TimeSpan.FromMilliseconds(180)) { EasingFunction = new QuadraticEase { EasingMode = EasingMode.EaseOut } });
    }
}
