using System.Text.RegularExpressions;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using Microsoft.EntityFrameworkCore;
namespace DonovanPOS.Infrastructure.Persistence;

public sealed class ProductQueries(Database database) : IProductQueries
{
    private Task<T> Read<T>(Func<PosDbContext, Task<T>> action, CancellationToken ct = default) => Task.Run(async () =>
    {
        await database.Gate.WaitAsync(ct); try { await using var db = database.Context(); return await action(db); } finally { database.Gate.Release(); }
    }, ct);
    public Task<ProductPage> Search(string search, int page, bool archived, string? category, string? stockFilter, CancellationToken ct) => Read(async db =>
    {
        Amount.Require(search.Length <= 200, "La búsqueda debe tener como máximo 200 caracteres.");
        IQueryable<Product> query = db.Products.AsNoTracking();
        if (!string.IsNullOrWhiteSpace(search))
        {
            var tokens = Regex.Matches(search, @"[\p{L}\p{N}]+").Select(x => x.Value).Take(12).ToArray();
            if (tokens.Length == 0) return new ProductPage([], 0);
            var match = string.Join(" AND ", tokens.Select(x => "\"" + x + "\"*")); var exact = search.Trim(); var sku = exact.ToUpperInvariant();
            query = db.Products.FromSqlInterpolated($"SELECT * FROM Product WHERE Id IN (SELECT rowid FROM ProductSearch WHERE ProductSearch MATCH {match}) OR Sku = {sku} OR Barcode = {exact}").AsNoTracking();
        }
        if (!archived) query = query.Where(x => x.Active);
        if (!string.IsNullOrWhiteSpace(category) && category != "Todos") query = query.Where(x => x.Category == category);
        if (stockFilter == "Por reabastecer") query = query.Where(x => x.StockUnits <= x.MinimumUnits);
        if (stockFilter == "Agotados") query = query.Where(x => x.StockUnits == 0);
        if (stockFilter == "Disponibles") query = query.Where(x => x.StockUnits > x.MinimumUnits);
        return new ProductPage(await query.OrderBy(x => x.Name).ThenBy(x => x.Id).Skip(Math.Max(0, page) * 100).Take(100).ToListAsync(ct), await query.CountAsync(ct));
    }, ct);
    public Task<List<string>> Categories() => Read(db => db.Products.Where(x => x.Active).Select(x => x.Category).Distinct().Order().ToListAsync());
    public Task<StockSummary> Summary() => Read(async db => new StockSummary(await db.Products.CountAsync(x => x.Active), await db.Products.CountAsync(x => x.Active && x.StockUnits > x.MinimumUnits), await db.Products.CountAsync(x => x.Active && x.StockUnits > 0 && x.StockUnits <= x.MinimumUnits), await db.Products.CountAsync(x => x.Active && x.StockUnits == 0)));
}
