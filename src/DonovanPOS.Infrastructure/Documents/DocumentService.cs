using System.Globalization;
using System.IO.Compression;
using ClosedXML.Excel;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using PdfSharp.Drawing;
using PdfSharp.Fonts;
using PdfSharp.Pdf;
namespace DonovanPOS.Infrastructure.Documents;

public sealed partial class DocumentService : IDocumentService
{
    private static readonly CultureInfo Mx = CultureInfo.GetCultureInfo("es-MX");
    static DocumentService() { GlobalFontSettings.UseWindowsFontsUnderWindows = true; }
    public Task<byte[]> ReadLogo(string path) => Task.Run(() =>
    {
        Amount.Require(File.Exists(path) && new FileInfo(path).Length <= 2 * 1024 * 1024, "El logotipo debe ser PNG o JPEG y menor de 2 MB.");
        var bytes = File.ReadAllBytes(path); using var image = XImage.FromStream(new MemoryStream(bytes));
        Amount.Require(image.PixelWidth <= 4096 && image.PixelHeight <= 4096, "El logotipo admite hasta 4096 × 4096 píxeles."); return bytes;
    });
    public Task Excel(string path, TableData table) => Task.Run(() =>
    {
        using var book = new XLWorkbook(); var sheet = book.AddWorksheet("Datos");
        for (var c = 0; c < table.Headers.Length; c++) sheet.Cell(1, c + 1).Value = table.Headers[c];
        for (var r = 0; r < table.Rows.Count; r++) for (var c = 0; c < table.Headers.Length; c++)
        {
            var value = table.Rows[r].ElementAtOrDefault(c) ?? ""; var cell = sheet.Cell(r + 2, c + 1);
            if (System.Text.RegularExpressions.Regex.IsMatch(table.Headers[c], "precio|importe|total|saldo|costo|impuesto|existencia|mínimo|cantidad|factor|descuento|cambio|efectivo|tarjeta|transferencia|crédito|utilidad|margen|ventas|devoluciones|abono", System.Text.RegularExpressions.RegexOptions.IgnoreCase)
                && decimal.TryParse(value, NumberStyles.Number | NumberStyles.AllowCurrencySymbol, Mx, out var number))
            { cell.Value = (double)number; cell.Style.NumberFormat.Format = value.Contains('$') ? "$#,##0.00" : "#,##0.####"; }
            else cell.Value = value;
        }
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
        if (ticketWidth is int width) CreateThermal(path, store, document, width); else CreateA4(path, store, document);
    });
    public Task ReportPdf(string path, StoreSettings store, TableData table) => Task.Run(() =>
    {
        using var pdf = new PdfDocument(); pdf.Info.Title = table.Title; pdf.Info.Author = store.StoreName;
        var font = new XFont("Arial", 9); var bold = new XFont("Arial", 9, XFontStyleEx.Bold);
        XGraphics? g = null; double y = 0; const double width = 765; var column = width / Math.Max(table.Headers.Length, 1);
        void NewPage()
        {
            g?.Dispose(); var page = pdf.AddPage(); page.Width = XUnit.FromPoint(841.89); page.Height = XUnit.FromPoint(595.28); g = XGraphics.FromPdfPage(page);
            g.DrawRectangle(Teal, 38, 22, 38, 4); y = 37; var heading = new XFont("Arial", 14, XFontStyleEx.Bold);
            foreach (var line in Wrap(g, store.StoreName + " · " + table.Title, heading, width - 75)) { g.DrawString(line, heading, Navy, new XRect(38, y, width - 75, 19), XStringFormats.TopLeft); y += 19; }
            if (store.LogoData is { Length: > 0 }) { using var logo = XImage.FromStream(new MemoryStream(store.LogoData)); var h = Math.Min(34d, 60d * logo.PixelHeight / logo.PixelWidth); g.DrawImage(logo, 743, 29, h * logo.PixelWidth / logo.PixelHeight, h); }
            y += 16; var headers = table.Headers.Select(x => Wrap(g, x, bold, column - 16).ToArray()).ToArray(); var headerHeight = headers.Max(x => x.Length) * 13 + 16;
            g.DrawRectangle(Navy, 38, y, width, headerHeight);
            for (var c = 0; c < headers.Length; c++) for (var i = 0; i < headers[c].Length; i++) g.DrawString(headers[c][i], bold, XBrushes.White, new XRect(46 + c * column, y + 8 + i * 13, column - 16, 14), XStringFormats.TopLeft);
            y += headerHeight + 8;
        }
        NewPage();
        foreach (var row in table.Rows)
        {
            var cells = row.Select(x => Wrap(g!, x, font, column - 8).ToArray()).ToArray(); var height = cells.Max(x => x.Length) * 12 + 10;
            if (y + height > 550) NewPage();
            for (var c = 0; c < Math.Min(cells.Length, table.Headers.Length); c++) for (var i = 0; i < cells[c].Length; i++) g!.DrawString(cells[c][i], font, Navy, new XRect(42 + c * column, y + 12 * i, column - 8, 14), XStringFormats.TopLeft);
            y += height; g!.DrawLine(new XPen(XColors.LightGray, .5), 38, y - 4, 803, y - 4);
        }
        if (table.Rows.Count == 0) g!.DrawString("Sin registros para este período.", font, XBrushes.Gray, 38, y + 20);
        g!.Dispose();
        for (var i = 0; i < pdf.PageCount; i++) { using var footer = XGraphics.FromPdfPage(pdf.Pages[i]); footer.DrawString($"Donovan POS Pro · Página {i + 1} de {pdf.PageCount}", font, XBrushes.DimGray, 38, 575); }
        pdf.Save(path);
    });
    private static IEnumerable<string> Wrap(XGraphics g, string text, XFont font, double width)
    {
        foreach (var paragraph in text.Replace("\r", "").Split('\n'))
        {
            var line = "";
            foreach (System.Text.RegularExpressions.Match match in System.Text.RegularExpressions.Regex.Matches(paragraph, @"\S+"))
            {
                var word = match.Value; var candidate = line.Length > 0 ? line + " " + word : word;
                if (g.MeasureString(candidate, font).Width <= width) { line = candidate; continue; }
                if (line.Length > 0) { yield return line; line = ""; }
                foreach (var character in word) { if (g.MeasureString(line + character, font).Width > width && line.Length > 0) { yield return line; line = ""; } line += character; }
            }
            yield return line;
        }
    }
}

