using System.Globalization;
using System.IO.Compression;
using ClosedXML.Excel;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using PdfSharp.Drawing;
using PdfSharp.Fonts;
using PdfSharp.Pdf;
namespace DonovanPOS.Infrastructure.Documents;

public sealed class DocumentService : IDocumentService
{
    private static readonly CultureInfo Mx = CultureInfo.GetCultureInfo("es-MX");
    static DocumentService() { GlobalFontSettings.UseWindowsFontsUnderWindows = true; }
    public Task Excel(string path, TableData table) => Task.Run(() =>
    {
        using var book = new XLWorkbook(); var sheet = book.AddWorksheet("Datos");
        for (var c = 0; c < table.Headers.Length; c++) sheet.Cell(1, c + 1).Value = table.Headers[c];
        for (var r = 0; r < table.Rows.Count; r++) for (var c = 0; c < table.Headers.Length; c++) sheet.Cell(r + 2, c + 1).Value = table.Rows[r].ElementAtOrDefault(c) ?? "";
        var header = sheet.Range(1, 1, 1, table.Headers.Length); header.Style.Fill.BackgroundColor = XLColor.FromHtml("123B44"); header.Style.Font.FontColor = XLColor.White; header.Style.Font.Bold = true;
        sheet.SheetView.FreezeRows(1); sheet.Range(1, 1, Math.Max(1, table.Rows.Count + 1), table.Headers.Length).SetAutoFilter(); sheet.Columns().AdjustToContents(1, Math.Min(table.Rows.Count + 1, 250));
        foreach (var column in sheet.ColumnsUsed()) column.Width = Math.Clamp(column.Width + 2, 12, 55);
        sheet.Row(1).Height = 28; book.SaveAs(path);
    });
    public Task<TableData> ReadExcel(string path) => Task.Run(() =>
    {
        Amount.Require(new FileInfo(path).Length <= 15 * 1024 * 1024, "El archivo debe ser menor de 15 MB.");
        using (var zip = ZipFile.OpenRead(path)) Amount.Require(zip.Entries.Sum(x => x.Length) <= 100 * 1024 * 1024, "El archivo descomprimido es demasiado grande.");
        using var book = new XLWorkbook(path); var sheet = book.Worksheets.First();
        var lastRow = sheet.LastRowUsed()?.RowNumber() ?? 1; var columns = sheet.LastColumnUsed()?.ColumnNumber() ?? 0;
        Amount.Require(lastRow <= 10001 && columns == ImportService.Headers.Length, "Usa la plantilla de productos, con un máximo de 10,000 filas.");
        string Cell(int r, int c)
        {
            var cell = sheet.Cell(r, c); Amount.Require(!cell.HasFormula, $"Fila {r}: reemplaza las fórmulas por valores antes de importar.");
            return cell.GetString().Trim();
        }
        var headers = Enumerable.Range(1, columns).Select(c => Cell(1, c)).ToArray(); List<string[]> rows = [];
        for (var r = 2; r <= lastRow; r++) { var row = Enumerable.Range(1, columns).Select(c => Cell(r, c)).ToArray(); if (row.Any(x => x.Length > 0)) rows.Add(row); }
        return new TableData("Productos", headers, rows);
    });
    public Task Pdf(string path, StoreSettings store, BusinessDocument document, int? ticketWidth = null) => Task.Run(() =>
    {
        using var pdf = new PdfDocument(); pdf.Info.Title = document.Title + " " + document.Folio; pdf.Info.Author = store.StoreName;
        var thermal = ticketWidth.HasValue; var width = thermal ? ticketWidth!.Value * 72d / 25.4 : 595.28;
        var margin = thermal ? 10 : 38; var contentWidth = width - 2 * margin;
        var size = thermal ? 8d : 10d; var regular = new XFont("Arial", size); var bold = new XFont("Arial", size, XFontStyleEx.Bold);
        XGraphics? gfx = null; PdfPage? page = null; double y = 0;
        void NewPage()
        {
            gfx?.Dispose(); page = pdf.AddPage(); page.Width = XUnit.FromPoint(width); page.Height = XUnit.FromPoint(thermal ? 800 : 841.89);
            gfx = XGraphics.FromPdfPage(page); y = margin;
        }
        void Line(string text, bool strong = false, double gap = 4)
        {
            var font = strong ? bold : regular;
            foreach (var line in Wrap(gfx!, text, font, contentWidth))
            {
                if (y > page!.Height.Point - margin - 24) NewPage();
                gfx!.DrawString(line, font, XBrushes.Black, new XRect(margin, y, contentWidth, size + 5), XStringFormats.TopLeft); y += size + 5;
            }
            y += gap;
        }
        NewPage();
        if (!string.IsNullOrWhiteSpace(store.LogoPath) && File.Exists(store.LogoPath))
        {
            using var logo = XImage.FromFile(store.LogoPath); var h = Math.Min(45, logo.PixelHeight * 90d / Math.Max(1, logo.PixelWidth));
            gfx!.DrawImage(logo, margin, y, h * logo.PixelWidth / Math.Max(1, logo.PixelHeight), h); y += h + 8;
        }
        Line(store.StoreName, true); if (store.Address.Length > 0) Line(store.Address);
        if (store.TaxId.Length > 0) Line("RFC: " + store.TaxId); if (store.Phone.Length > 0) Line("Tel. " + store.Phone);
        y += 8; Line(document.Title.ToUpper(Mx), true); Line(document.Folio + "  ·  " + document.Date.ToString("dd/MM/yyyy HH:mm", Mx));
        Line("Cliente: " + document.Customer); y += 6;
        foreach (var item in document.Lines)
        {
            Line(item.Description, true, 0);
            Line($"{item.Quantity} × {item.Price.ToString("C2", Mx)}    {item.Total.ToString("C2", Mx)}", false, 8);
        }
        y += 4; Line("Subtotal: " + (document.Total - document.Tax).ToString("C2", Mx)); Line("IVA incluido: " + document.Tax.ToString("C2", Mx));
        Line("TOTAL MXN  " + document.Total.ToString("C2", Mx), true); y += 6;
        if (document.Notes.Length > 0) Line(document.Notes);
        Line(store.TicketFooter); Line("Este documento no es un CFDI.");
        if (thermal && pdf.PageCount == 1) page!.Height = XUnit.FromPoint(Math.Max(150, y + margin));
        gfx!.Dispose(); pdf.Save(path);
    });
    public Task ReportPdf(string path, StoreSettings store, TableData table) => Task.Run(() =>
    {
        using var pdf = new PdfDocument(); pdf.Info.Title = table.Title; pdf.Info.Author = store.StoreName;
        var font = new XFont("Arial", 9); var bold = new XFont("Arial", 9, XFontStyleEx.Bold);
        XGraphics? g = null; double y = 0; const double width = 765; var column = width / Math.Max(table.Headers.Length, 1);
        void NewPage()
        {
            g?.Dispose(); var page = pdf.AddPage(); page.Width = XUnit.FromPoint(841.89); page.Height = XUnit.FromPoint(595.28); g = XGraphics.FromPdfPage(page);
            g.DrawString(store.StoreName + " · " + table.Title, new XFont("Arial", 14, XFontStyleEx.Bold), XBrushes.Black, 38, 32); y = 50;
            for (var c = 0; c < table.Headers.Length; c++) g.DrawString(table.Headers[c], bold, XBrushes.Black, new XRect(38 + c * column, y, column - 6, 32), XStringFormats.TopLeft);
            y += 34;
        }
        NewPage();
        foreach (var row in table.Rows)
        {
            var cells = row.Select(x => Wrap(g!, x, font, column - 8).ToArray()).ToArray(); var height = cells.Max(x => x.Length) * 12 + 10;
            if (y + height > 550) NewPage();
            for (var c = 0; c < Math.Min(cells.Length, table.Headers.Length); c++) for (var i = 0; i < cells[c].Length; i++) g!.DrawString(cells[c][i], font, XBrushes.Black, new XRect(38 + c * column, y + 12 * i, column - 8, 14), XStringFormats.TopLeft);
            y += height; g!.DrawLine(new XPen(XColors.LightGray, .5), 38, y - 4, 803, y - 4);
        }
        if (table.Rows.Count == 0) g!.DrawString("Sin registros para este período.", font, XBrushes.Gray, 38, y + 20);
        g!.Dispose(); pdf.Save(path);
    });
    private static IEnumerable<string> Wrap(XGraphics g, string text, XFont font, double width)
    {
        foreach (var paragraph in text.Replace("\r", "").Split('\n'))
        {
            var line = "";
            foreach (var character in paragraph)
            {
                if (g.MeasureString(line + character, font).Width > width && line.Length > 0) { yield return line; line = ""; }
                line += character;
            }
            yield return line;
        }
    }
}
