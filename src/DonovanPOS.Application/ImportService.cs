using System.Globalization;
using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed record ImportRow(int Row, Product? Product, string Error);
public sealed record ImportPreview(List<ImportRow> Rows) { public bool Valid => Rows.Count > 0 && Rows.All(x => x.Error.Length == 0); }
public sealed class ImportService(IDataStore store, IDocumentService documents, UserSession session, IClock clock)
{
    public static readonly string[] Headers = ["Clave", "Código de barras", "Descripción", "Categoría", "Marca", "Medidas", "Unidad", "Costo neto", "Precio con IVA", "Mayoreo con IVA", "Existencia", "Mínimo", "IVA %", "Presentación", "Unidades por presentación"];
    public async Task<ImportPreview> Preview(string path)
    {
        session.Require(Permission.Catalog);
        var table = await documents.ReadExcel(path);
        Amount.Require(table.Headers.SequenceEqual(Headers), "Las columnas no coinciden. Descarga la plantilla y conserva sus encabezados.");
        var existing = await store.Read(db => db.List<Product>(take: int.MaxValue));
        var skus = existing.Select(x => x.Sku).ToHashSet(StringComparer.OrdinalIgnoreCase);
        var codes = existing.Where(x => x.Barcode is not null).Select(x => x.Barcode!).ToHashSet(StringComparer.Ordinal);
        List<ImportRow> rows = [];
        for (var i = 0; i < table.Rows.Count; i++)
        {
            var values = table.Rows[i];
            try
            {
                decimal Number(int n, decimal fallback = 0) => string.IsNullOrWhiteSpace(values[n]) ? fallback : decimal.Parse(values[n], NumberStyles.Number, CultureInfo.GetCultureInfo("es-MX"));
                var p = new Product { Sku = values[0], Barcode = values[1], Name = values[2], Category = values[3], Brand = values[4], Size = values[5], Unit = Enum.Parse<Unit>(values[6], true), CostCents = string.IsNullOrWhiteSpace(values[7]) ? null : Amount.Cents(Number(7)), PriceCents = Amount.Cents(Number(8)), WholesaleCents = Amount.Cents(Number(9)), StockUnits = Amount.Units(Number(10)), MinimumUnits = Amount.Units(Number(11)), TaxBasisPoints = checked((int)(Number(12) * 100)), PackName = values[13], PackUnits = Amount.Units(Number(14, 1)) };
                p.Validate(); if (p.StockUnits > 0) Amount.QuantityFor(p.Unit, p.StockUnits);
                Amount.Require(skus.Add(p.Sku), "La clave ya existe en el catálogo o en este archivo.");
                Amount.Require(p.Barcode is null || codes.Add(p.Barcode), "El código de barras está duplicado.");
                rows.Add(new(i + 2, p, ""));
            }
            catch (Exception ex) when (ex is FormatException or OverflowException or ArgumentException or RuleException) { rows.Add(new(i + 2, null, ex is RuleException ? ex.Message : "Revisa los números y la unidad (Pieza, Metro, Kilogramo, Litro, Caja o Rollo).")); }
        }
        return new ImportPreview(rows);
    }
    public Task<int> Commit(ImportPreview preview)
    {
        var actor = session.Require(Permission.Catalog); Amount.Require(preview.Valid, "Corrige todas las filas antes de importar.");
        return store.Write(async db =>
        {
            foreach (var row in preview.Rows)
            {
                var p = row.Product!.CopyForImport(); p.Validate();
                Amount.Require(await db.Count<Product>(x => x.Sku == p.Sku || (p.Barcode != null && x.Barcode == p.Barcode)) == 0, $"Fila {row.Row}: la clave o código ya existe. Genera otra vista previa.");
                var initial = p.StockUnits; p.StockUnits = 0; db.Add(p); await db.Flush();
                if (initial > 0) Operations.Stock(db, clock, actor.Id, p, initial, "Importación", p.Sku);
            }
            Operations.Audit(db, clock, actor.Id, "Importar productos", preview.Rows.Count.ToString()); return preview.Rows.Count;
        });
    }
    public Task Template(string path) { session.Require(Permission.Catalog); return documents.Excel(path, new TableData("Productos", Headers, [])); }
    public async Task Export(string path)
    {
        session.Require(Permission.Catalog); var products = await store.Read(db => db.List<Product>(take: int.MaxValue));
        var culture = CultureInfo.GetCultureInfo("es-MX");
        string N(decimal n) => n.ToString("0.####", culture);
        var rows = products.Select(p => new[] { p.Sku, p.Barcode ?? "", p.Name, p.Category, p.Brand, p.Size, p.Unit.ToString(), p.CostCents is long c ? N(Amount.Pesos(c)) : "", N(p.UnitPrice), N(Amount.Pesos(p.WholesaleCents)), N(p.Stock), N(p.Minimum), N(p.TaxBasisPoints / 100m), p.PackName, N(Amount.Quantity(p.PackUnits)) }).ToList();
        await documents.Excel(path, new TableData("Productos", Headers, rows));
    }
}
