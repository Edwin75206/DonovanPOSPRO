using DonovanPOS.Application;
using DonovanPOS.Domain;
using PdfSharp.Drawing;
using PdfSharp.Pdf;
using ZXing;
using ZXing.OneD;
namespace DonovanPOS.Infrastructure.Documents;
public sealed partial class DocumentService
{
    private static readonly XSolidBrush Navy = new(XColor.FromArgb(12, 39, 50));
    private static readonly XSolidBrush Teal = new(XColor.FromArgb(0, 155, 151));
    private static readonly XSolidBrush Pale = new(XColor.FromArgb(241, 246, 249));
    private static void CreateA4(string path, StoreSettings store, BusinessDocument document)
    {
        using var pdf = new PdfDocument(); pdf.Info.Title = document.Title + " " + document.Folio; pdf.Info.Author = store.StoreName;
        var font = new XFont("Arial", 9); var bold = new XFont("Arial", 9, XFontStyleEx.Bold); var small = new XFont("Arial", 8); var title = new XFont("Arial", 22, XFontStyleEx.Bold);
        XGraphics? g = null; double y = 0; var widths = new double[] { 270, 75, 83, 91 }; const double left = 38, right = 557;
        void Text(string text, XFont f, XBrush brush, double x, double top, double width, bool alignRight = false) => g!.DrawString(text, f, brush, new XRect(x, top, width, f.Size + 5), alignRight ? XStringFormats.TopRight : XStringFormats.TopLeft);
        void TableHead()
        {
            g!.DrawRectangle(Navy, left, y, 519, 27); var x = left;
            foreach (var (label, i) in new[] { "PRODUCTO / DESCRIPCIÓN", "CANTIDAD", "P. UNITARIO", "IMPORTE" }.Select((s, i) => (s, i))) { Text(label, small, XBrushes.White, x + 10, y + 9, widths[i] - 20, i >= 2); x += widths[i]; } y += 27;
        }
        void NewPage(bool first)
        {
            g?.Dispose(); var page = pdf.AddPage(); g = XGraphics.FromPdfPage(page); g.DrawRectangle(Teal, 38, 30, 42, 4);
            var nameFont = g.MeasureString(store.StoreName, title).Width <= 320 ? title : new XFont("Arial", 15, XFontStyleEx.Bold);
            var nameY = 48d; foreach (var line in Wrap(g, store.StoreName, nameFont, 320)) { Text(line, nameFont, Navy, 38, nameY, 320); nameY += nameFont.Size + 3; }
            Text(document.Title.ToUpperInvariant(), bold, Teal, 378, 51, 179, true);
            Text(document.Folio, new XFont("Arial", 17, XFontStyleEx.Bold), Navy, 378, 71, 179, true); Text(document.Date.ToString("dd/MM/yyyy HH:mm", Mx), small, XBrushes.DimGray, 378, 96, 179, true);
            var info = string.Join("  ·  ", new[] { store.Address, store.Phone.Length > 0 ? "Tel. " + store.Phone : "", store.TaxId.Length > 0 ? "RFC " + store.TaxId : "" }.Where(x => x.Length > 0));
            var infoY = Math.Max(80d, nameY + 8); foreach (var line in Wrap(g, info, small, 320)) { Text(line, small, XBrushes.DimGray, 38, infoY, 320); infoY += 12; }
            y = Math.Max(125, infoY + 14);
            if (first)
            {
                g.DrawRectangle(Pale, left, y, 519, 51); Text("PREPARADO PARA", small, Teal, left + 14, y + 10, 350); Text(document.Customer, bold, Navy, left + 14, y + 26, 420);
                if (store.LogoData is { Length: > 0 }) { using var logo = XImage.FromStream(new MemoryStream(store.LogoData)); var h = 32d; var w = Math.Min(55, h * logo.PixelWidth / logo.PixelHeight); g.DrawImage(logo, right - w - 10, y + 10, w, h); }
                y += 68;
            }
            TableHead();
        }
        NewPage(true); var alternate = false;
        foreach (var item in document.Lines)
        {
            var description = Wrap(g!, item.Description, font, widths[0] - 22).ToArray(); var quantity = Wrap(g!, item.Quantity, font, widths[1] - 16).ToArray(); var height = Math.Max(description.Length, quantity.Length) * 13 + 18;
            if (y + height > 750) NewPage(false);
            if (alternate) g!.DrawRectangle(Pale, left, y, 519, height); alternate = !alternate;
            for (var i = 0; i < description.Length; i++) Text(description[i], font, Navy, left + 10, y + 9 + i * 13, widths[0] - 22);
            for (var i = 0; i < quantity.Length; i++) Text(quantity[i], font, Navy, left + widths[0] + 8, y + 9 + i * 13, widths[1] - 16);
            Text(item.Price.ToString("C2", Mx), font, Navy, left + widths[0] + widths[1], y + 9, widths[2] - 10, true); Text(item.Total.ToString("C2", Mx), bold, Navy, right - widths[3], y + 9, widths[3] - 10, true);
            y += height; g!.DrawLine(new XPen(XColors.LightGray, .3), left, y, right, y);
        }
        if (y + 125 > 760) NewPage(false); y += 20;
        Text("Subtotal sin IVA", font, XBrushes.DimGray, 330, y, 120); Text((document.Total - document.Tax).ToString("C2", Mx), font, Navy, 450, y, 107, true); y += 20;
        Text("IVA incluido", font, XBrushes.DimGray, 330, y, 120); Text(document.Tax.ToString("C2", Mx), font, Navy, 450, y, 107, true); y += 23;
        g!.DrawRectangle(Navy, 320, y, 237, 46); Text("TOTAL MXN", bold, XBrushes.White, 334, y + 17, 80); Text(document.Total.ToString("C2", Mx), new XFont("Arial", 18, XFontStyleEx.Bold), XBrushes.White, 410, y + 12, 132, true); y += 68;
        foreach (var line in Wrap(g, document.Notes + "\n" + store.TicketFooter, font, 519)) { if (y > 770) { NewPage(false); y += 12; } Text(line, font, XBrushes.DimGray, left, y, 519); y += 14; }
        g.Dispose();
        for (var i = 0; i < pdf.PageCount; i++) { using var footer = XGraphics.FromPdfPage(pdf.Pages[i]); footer.DrawLine(new XPen(XColors.LightGray, .5), 38, 801, 557, 801); footer.DrawString("Este documento no es un CFDI.  ·  " + store.StoreName, small, XBrushes.DimGray, 38, 817); footer.DrawString($"{i + 1} / {pdf.PageCount}", small, XBrushes.DimGray, new XRect(500, 807, 57, 16), XStringFormats.TopRight); }
        pdf.Save(path);
    }
    public Task Labels(string path, StoreSettings store, IReadOnlyList<BarcodeLabel> labels) => Task.Run(() =>
    {
        Amount.Require(labels.Count > 0 && labels.All(x => x.Copies > 0) && labels.Sum(x => x.Copies) <= 500, "Genera entre 1 y 500 etiquetas por archivo.");
        using var pdf = new PdfDocument(); pdf.Info.Title = "Etiquetas de productos"; pdf.Info.Author = store.StoreName;
        XGraphics? g = null; var index = 0; var font = new XFont("Arial", 8); var bold = new XFont("Arial", 10, XFontStyleEx.Bold);
        foreach (var label in labels)
        {
            Amount.Require(label.Code.Length is > 0 and <= 40 && label.Code.All(c => c is >= ' ' and <= '~'), "Usa una clave o código de barras de hasta 40 caracteres ASCII.");
            var barcode = new Code128Writer().encode(label.Code, BarcodeFormat.CODE_128, 0, 1, new Dictionary<EncodeHintType, object> { [EncodeHintType.MARGIN] = 20 });
            Amount.Require(245d / barcode.Width >= .65, "Este código es demasiado largo para la etiqueta. Asigna un código numérico más corto.");
            for (var copy = 0; copy < label.Copies; copy++, index++)
            {
                if (index % 16 == 0) { g?.Dispose(); var page = pdf.AddPage(); g = XGraphics.FromPdfPage(page); }
                var x = 28 + (index % 2) * 273d; var y = 26 + (index % 16 / 2) * 97d;
                g!.DrawRectangle(new XPen(XColors.LightGray, .4), x, y, 266, 90);
                var description = Wrap(g, label.Name, font, 175).Take(2).ToArray(); for (var i = 0; i < description.Length; i++) g.DrawString(description[i], font, XBrushes.Black, new XRect(x + 9, y + 7 + i * 10, 175, 12), XStringFormats.TopLeft);
                g.DrawString(Amount.Pesos(label.PriceCents).ToString("C2", Mx), bold, XBrushes.Black, new XRect(x + 180, y + 9, 76, 14), XStringFormats.TopRight);
                var module = 245d / barcode.Width; for (var bit = 0; bit < barcode.Width; bit++) if (barcode[bit, 0]) g.DrawRectangle(XBrushes.Black, x + 10 + bit * module, y + 35, module, 30);
                g.DrawString(label.Code, font, XBrushes.Black, new XRect(x + 10, y + 69, 245, 12), XStringFormats.TopCenter);
            }
        }
        g?.Dispose(); pdf.Save(path);
    });
}
