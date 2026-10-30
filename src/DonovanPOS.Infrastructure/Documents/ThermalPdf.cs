using DonovanPOS.Application;
using DonovanPOS.Domain;
using PdfSharp.Drawing;
using PdfSharp.Pdf;
namespace DonovanPOS.Infrastructure.Documents;
public sealed partial class DocumentService
{
    private static void CreateThermal(string path, StoreSettings store, BusinessDocument document, int millimeters)
    {
        Amount.Require(millimeters is 58 or 80, "Usa un ancho de ticket de 58 u 80 mm.");
        var width = millimeters * 72d / 25.4; const double margin = 10; var contentWidth = width - 2 * margin;
        var font = new XFont("Arial", 8); var bold = new XFont("Arial", 8, XFontStyleEx.Bold); List<(string Text, bool Bold, double Gap)> rows = [];
        void Add(string text, bool strong = false, double gap = 4) { if (!string.IsNullOrWhiteSpace(text)) rows.Add((text, strong, gap)); }
        Add(store.StoreName, true); Add(store.Address); if (store.TaxId.Length > 0) Add("RFC: " + store.TaxId); Add(store.Phone);
        Add(document.Title.ToUpperInvariant(), true, 7); Add(document.Folio + " · " + document.Date.ToString("dd/MM/yyyy HH:mm", Mx)); Add("Cliente: " + document.Customer, false, 9);
        foreach (var item in document.Lines) { Add(item.Description, true, 0); Add($"{item.Quantity} × {item.Price.ToString("C2", Mx)} = {item.Total.ToString("C2", Mx)}", false, 8); }
        Add("Subtotal sin IVA: " + (document.Total - document.Tax).ToString("C2", Mx)); Add("IVA incluido: " + document.Tax.ToString("C2", Mx)); Add("TOTAL MXN " + document.Total.ToString("C2", Mx), true, 10); Add(document.Notes); Add(store.TicketFooter); Add("Este documento no es un CFDI.");
        using var logo = store.LogoData is { Length: > 0 } ? XImage.FromStream(new MemoryStream(store.LogoData)) : null; var logoHeight = logo is null ? 0 : Math.Min(40, logo.PixelHeight * 85d / logo.PixelWidth);
        using var measure = XGraphics.CreateMeasureContext(new XSize(width, 800), XGraphicsUnit.Point, XPageDirection.Downwards);
        var wrapped = rows.Select(x => (Lines: Wrap(measure, x.Text, x.Bold ? bold : font, contentWidth).ToArray(), x.Bold, x.Gap)).ToList();
        // Page geometry must be final before XGraphics establishes its top-left transform.
        var naturalHeight = margin * 2 + (logo is null ? 0 : logoHeight + 8) + wrapped.Sum(x => x.Lines.Length * 13 + x.Gap) + 24;
        var pageHeight = Math.Clamp(naturalHeight, 150, 800);
        using var pdf = new PdfDocument(); pdf.Info.Title = document.Title + " " + document.Folio; pdf.Info.Author = store.StoreName;
        XGraphics? g = null; double y = margin;
        void NewPage() { g?.Dispose(); var page = pdf.AddPage(); page.Width = XUnit.FromPoint(width); page.Height = XUnit.FromPoint(pageHeight); g = XGraphics.FromPdfPage(page); y = margin; }
        NewPage(); if (logo is not null) { var logoWidth = logoHeight * logo.PixelWidth / logo.PixelHeight; g!.DrawImage(logo, (width - logoWidth) / 2, y, logoWidth, logoHeight); y += logoHeight + 8; }
        foreach (var row in wrapped)
        {
            foreach (var line in row.Lines) { if (y + 13 > pageHeight - margin) NewPage(); g!.DrawString(line, row.Bold ? bold : font, XBrushes.Black, new XRect(margin, y, contentWidth, 13), XStringFormats.TopLeft); y += 13; } y += row.Gap;
        }
        g!.Dispose(); pdf.Save(path);
    }
}
