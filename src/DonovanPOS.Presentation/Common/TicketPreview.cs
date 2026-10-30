using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using DonovanPOS.Application;
using DonovanPOS.Domain;
namespace DonovanPOS.Presentation.Common;
public sealed partial class Dialogs
{
    public void Print(StoreSettings store, BusinessDocument document)
    {
        var width = store.TicketWidthMm * 96d / 25.4;
        var flow = new FlowDocument { PageWidth = width, ColumnWidth = width, PagePadding = new Thickness(8), FontFamily = new FontFamily("Arial"), FontSize = 10, Background = Brushes.White, Foreground = Brushes.Black };
        void P(string text, bool bold = false) { if (string.IsNullOrWhiteSpace(text)) return; flow.Blocks.Add(new Paragraph(new Run(text)) { FontWeight = bold ? FontWeights.Bold : FontWeights.Normal, Margin = new Thickness(0, 0, 0, 6) }); }
        if (store.LogoData is { Length: > 0 })
        {
            using var stream = new MemoryStream(store.LogoData); var logo = new BitmapImage(); logo.BeginInit(); logo.CacheOption = BitmapCacheOption.OnLoad; logo.StreamSource = stream; logo.EndInit(); logo.Freeze();
            flow.Blocks.Add(new BlockUIContainer(new Image { Source = logo, Height = 42, MaxWidth = width - 16, Stretch = Stretch.Uniform, HorizontalAlignment = HorizontalAlignment.Center }));
        }
        P(store.StoreName, true); P(store.Address); P(store.Phone); if (store.TaxId.Length > 0) P("RFC: " + store.TaxId);
        P(document.Title + " · " + document.Folio, true); P(document.Date.ToString("dd/MM/yyyy HH:mm")); P(document.Customer);
        foreach (var item in document.Lines) { P(item.Description, true); P($"{item.Quantity} × {item.Price:C2} = {item.Total:C2}"); }
        P($"TOTAL MXN {document.Total:C2}", true); P($"IVA incluido {document.Tax:C2}"); P(document.Notes); P(store.TicketFooter); P("Este documento no es un CFDI.");
        var preview = new Window { Title = "Vista previa · " + document.Folio, Owner = System.Windows.Application.Current.MainWindow, Width = 560, Height = 760, MaxHeight = SystemParameters.WorkArea.Height, WindowStartupLocation = WindowStartupLocation.CenterOwner };
        var root = new DockPanel { Margin = new Thickness(22) }; var bar = new DockPanel { Margin = new Thickness(0, 0, 0, 16) }; var button = new Button { Content = "Elegir impresora e imprimir", Style = (Style)preview.FindResource("PrimaryButton") }; DockPanel.SetDock(button, Dock.Right); bar.Children.Add(button); bar.Children.Add(new TextBlock { Text = $"Ticket {store.TicketWidthMm} mm", FontWeight = FontWeights.SemiBold, VerticalAlignment = VerticalAlignment.Center }); DockPanel.SetDock(bar, Dock.Top); root.Children.Add(bar);
        var viewer = new FlowDocumentScrollViewer { Document = flow, Zoom = 140, MinZoom = 75, MaxZoom = 200, IsToolBarVisible = true, Background = Brushes.White, VerticalScrollBarVisibility = ScrollBarVisibility.Auto }; root.Children.Add(viewer); preview.Content = root;
        button.Click += (_, _) =>
        {
            try
            {
                var dialog = new PrintDialog(); if (!string.IsNullOrWhiteSpace(store.PrinterName)) { using var server = new System.Printing.LocalPrintServer(); dialog.PrintQueue = server.GetPrintQueues().FirstOrDefault(x => x.Name == store.PrinterName) ?? dialog.PrintQueue; }
                if (dialog.ShowDialog() != true) return;
                var paginator = ((IDocumentPaginatorSource)flow).DocumentPaginator; paginator.PageSize = new Size(Math.Min(width, dialog.PrintableAreaWidth), dialog.PrintableAreaHeight); dialog.PrintDocument(paginator, document.Folio); preview.Close();
            }
            catch (Exception ex) { ui.Notify(ui.Error(ex), true); }
        };
        preview.ShowDialog();
    }
}
