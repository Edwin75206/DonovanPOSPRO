using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Media;
using System.Windows.Threading;
namespace DonovanPOS.Presentation.Common;

public static class Icon
{
    public static readonly DependencyProperty GlyphProperty = DependencyProperty.RegisterAttached("Glyph", typeof(string), typeof(Icon), new PropertyMetadata(""));
    public static string GetGlyph(DependencyObject obj) => (string)obj.GetValue(GlyphProperty);
    public static void SetGlyph(DependencyObject obj, string value) => obj.SetValue(GlyphProperty, value);
}
public sealed class CategoryIcon : Control
{
    public static readonly DependencyProperty CategoryProperty = DependencyProperty.Register(nameof(Category), typeof(string), typeof(CategoryIcon), new PropertyMetadata(""));
    public string Category { get => (string)GetValue(CategoryProperty); set => SetValue(CategoryProperty, value); }
}
public sealed class StockBadge : Control
{
    public static readonly DependencyProperty StatusProperty = DependencyProperty.Register(nameof(Status), typeof(string), typeof(StockBadge), new PropertyMetadata("Disponible"));
    public string Status { get => (string)GetValue(StatusProperty); set => SetValue(StatusProperty, value); }
}
public sealed class CategoryColorConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
    {
        var category = value?.ToString()?.ToLowerInvariant() ?? "";
        var colors = category.Contains("eléc") || category.Contains("elec") ? ("#E7F3FE", "#2279BE") : category.Contains("plom") ? ("#E1F5F4", "#118A89") : category.Contains("pint") ? ("#F0EAFB", "#8C63B7") : category.Contains("torn") ? ("#FDF1D9", "#B88921") : category.Contains("segur") ? ("#E7F4E9", "#559E6B") : ("#EAF0F5", "#476C85");
        return new SolidColorBrush((Color)ColorConverter.ConvertFromString(parameter?.ToString() == "ink" ? colors.Item2 : colors.Item1));
    }
    public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture) => Binding.DoNothing;
}
public sealed class CategoryGeometryConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
    {
        var category = value?.ToString()?.ToLowerInvariant() ?? "";
        var path = category.Contains("eléc") || category.Contains("elec") ? "M13 2 L4 14 H11 L9 23 L21 9 H13 Z" :
            category.Contains("plom") ? "M3 5 H13 Q18 5 18 10 V14 H22 V21 H12 V11 H3 Z M3 3 V13 M10 17 H24" :
            category.Contains("pint") ? "M3 3 H21 V10 H3 Z M21 6 H24 V14 H12 V18 M9 18 H15 V25 H9 Z" :
            category.Contains("torn") ? "M6 4 L10 0 L24 14 L20 18 Z M11 9 L1 19 L5 23 L15 13 M4 16 L9 21 M7 13 L12 18" :
            category.Contains("segur") ? "M12 1 L23 5 V13 Q23 20 12 25 Q1 20 1 13 V5 Z M6 12 L10 16 L18 8" :
            "M4 4 L10 1 L17 8 L13 12 L24 23 L20 27 L9 16 L5 20 L1 16 L6 11 L2 7 Z";
        return Geometry.Parse(path);
    }
    public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture) => Binding.DoNothing;
}
/// <summary>Display-only integer interpolation. Financial calculations continue to use domain cents.</summary>
public sealed class AnimatedAmount : TextBlock
{
    public static readonly DependencyProperty ValueProperty = DependencyProperty.Register(nameof(Value), typeof(long), typeof(AnimatedAmount), new PropertyMetadata(0L, Changed));
    public static readonly DependencyProperty CurrencyProperty = DependencyProperty.Register(nameof(Currency), typeof(bool), typeof(AnimatedAmount), new PropertyMetadata(true, Changed));
    public long Value { get => (long)GetValue(ValueProperty); set => SetValue(ValueProperty, value); }
    public bool Currency { get => (bool)GetValue(CurrencyProperty); set => SetValue(CurrencyProperty, value); }
    private DispatcherTimer? timer; private long displayed;
    public AnimatedAmount() { Loaded += (_, _) => Update(); Unloaded += (_, _) => timer?.Stop(); Text = "$0.00"; }
    private static void Changed(DependencyObject d, DependencyPropertyChangedEventArgs e) => ((AnimatedAmount)d).Update();
    private void Update()
    {
        timer?.Stop(); var target = Value;
        if (!IsLoaded || ThemeManager.ReducedMotion || !SystemParameters.ClientAreaAnimation) { displayed = target; Show(); return; }
        var start = displayed; var step = 0; timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(16) };
        timer.Tick += (_, _) => { step++; displayed = checked((long)(start + ((decimal)target - start) * step / 12)); Show(); if (step >= 12) timer.Stop(); }; timer.Start();
    }
    private void Show() => Text = Currency ? Mx.Money(displayed) : displayed.ToString("N0", Mx.Culture);
}
