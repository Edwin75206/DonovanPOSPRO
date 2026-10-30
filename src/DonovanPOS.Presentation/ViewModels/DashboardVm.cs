using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
using LiveChartsCore;
using LiveChartsCore.SkiaSharpView;
using LiveChartsCore.SkiaSharpView.Painting;
using SkiaSharp;
namespace DonovanPOS.Presentation.ViewModels;

public sealed partial class DashboardVm(IReportingService reports, IClock clock, UiState ui) : PageVm(ui)
{
    public override string Title => "Tu negocio, de un vistazo";
    [ObservableProperty] private DashboardData? data;
    [ObservableProperty] private DateTime from = clock.Now.Date.AddDays(-6);
    [ObservableProperty] private DateTime to = clock.Now.Date;
    [ObservableProperty] private ISeries[] series = [];
    [ObservableProperty] private ISeries[] categorySeries = [];
    [ObservableProperty] private ISeries[] methodSeries = [];
    [ObservableProperty] private Axis[] xAxes = [];
    public Axis[] YAxes { get; } = [new Axis { Labeler = value => value.ToString("C0", Mx.Culture), TextSize = 11, LabelsPaint = new SolidColorPaint(SKColor.Parse("8198A2")), SeparatorsPaint = new SolidColorPaint(SKColor.Parse("DEE6E8")) { StrokeThickness = 1 } }];
    public string Today => Mx.Money(Data?.TodayCents ?? 0);
    public string Week => "Últimos 7 días  " + Mx.Money(Data?.WeekCents ?? 0);
    public string Month => "Este mes  " + Mx.Money(Data?.MonthCents ?? 0);
    public string Cash => Mx.Money(Data?.CashCents ?? 0);
    public string Credit => Mx.Money(Data?.ReceivableCents ?? 0);
    public string Period => Mx.Money(Data?.PeriodCents ?? 0);
    public string Profit => Data?.ProfitCents is long cents ? Mx.Money(cents) : "Faltan costos";
    public string Comparison => Data is null || Data.PreviousPeriodCents == 0 ? "Sin base para comparar" : $"{(Data.PeriodCents - Data.PreviousPeriodCents) / (decimal)Data.PreviousPeriodCents:+0.0%;-0.0%;0%} frente al período anterior";
    public bool Empty => Data is null || Data.Trend.All(x => x.Value == 0);
    public decimal TopMax => Data?.TopProducts.Select(x => x.Value).DefaultIfEmpty(1).Max() ?? 1;
    public bool NoLowStock => Data?.LowStock.Count == 0;
    public bool NoRecent => Data?.Recent.Count == 0;
    public TimeSpan AnimationSpeed => ThemeManager.ReducedMotion || !System.Windows.SystemParameters.ClientAreaAnimation ? TimeSpan.Zero : TimeSpan.FromMilliseconds(200);
    public override Task Load() => Refresh();
    [RelayCommand] private Task Refresh() => Run(async () =>
    {
        Data = await reports.Dashboard(From, To);
        var teal = SKColor.Parse("159A9D");
        Series = [new LineSeries<decimal> { Name = "Ventas netas MXN", Values = Data.Trend.Select(x => x.Value).ToArray(), Stroke = new SolidColorPaint(teal, 3), Fill = new SolidColorPaint(teal.WithAlpha(20)), GeometrySize = 7, GeometryFill = new SolidColorPaint(teal), GeometryStroke = new SolidColorPaint(teal), LineSmoothness = .25 }];
        XAxes = [new Axis { Labels = Data.Trend.Select(x => x.Label).ToArray(), TextSize = 11, LabelsPaint = new SolidColorPaint(SKColor.Parse("8198A2")), MinStep = 1, ForceStepToMin = true, SeparatorsPaint = null }];
        var colors = new[] { "159A9D", "D1AB64", "5473A5", "729E89", "B78973", "779DAE", "8E8DA6", "C4AE8E" };
        CategorySeries = Data.Categories.Select((x, i) => (ISeries)new PieSeries<decimal> { Name = x.Label, Values = [x.Value], InnerRadius = 48, Fill = new SolidColorPaint(SKColor.Parse(colors[i % colors.Length])), Stroke = null }).ToArray();
        MethodSeries = Data.Methods.Select((x, i) => (ISeries)new PieSeries<decimal> { Name = x.Label, Values = [x.Value], InnerRadius = 40, Fill = new SolidColorPaint(SKColor.Parse(colors[i % colors.Length])), Stroke = null }).ToArray();
        foreach (var property in new[] { nameof(Today), nameof(Week), nameof(Month), nameof(Cash), nameof(Credit), nameof(Period), nameof(Profit), nameof(Comparison), nameof(Empty), nameof(NoLowStock), nameof(NoRecent), nameof(AnimationSpeed), nameof(TopMax) }) OnPropertyChanged(property);
    });
    [RelayCommand] private async Task PeriodPreset(string days) { To = clock.Now.Date; From = days == "month" ? new(To.Year, To.Month, 1) : To.AddDays(-(int.Parse(days) - 1)); await Refresh(); }
}
