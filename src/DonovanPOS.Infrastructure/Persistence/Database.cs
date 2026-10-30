using System.Linq.Expressions;
using DonovanPOS.Application;
using DonovanPOS.Domain;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
namespace DonovanPOS.Infrastructure.Persistence;

public sealed class Database(string directory) : IDataStore
{
    public string DirectoryPath { get; } = Path.GetFullPath(directory);
    public string FilePath => Path.Combine(DirectoryPath, "donovan.db");
    internal SemaphoreSlim Gate { get; } = new(1, 1);
    public PosDbContext Context() => new(new DbContextOptionsBuilder<PosDbContext>().UseSqlite(new SqliteConnectionStringBuilder { DataSource = FilePath, ForeignKeys = true, DefaultTimeout = 15, Pooling = false }.ToString()).Options);
    public Task Initialize() => Task.Run(async () =>
    {
        Directory.CreateDirectory(DirectoryPath);
        await using var db = Context();
        await db.Database.MigrateAsync();
        await db.Database.ExecuteSqlRawAsync("PRAGMA journal_mode=WAL;");
        await db.Database.ExecuteSqlRawAsync("PRAGMA synchronous=FULL;");
        await db.Database.ExecuteSqlRawAsync("CREATE UNIQUE INDEX IF NOT EXISTS IX_CashSession_OneOpen ON CashSession((1)) WHERE ClosedAt IS NULL;");
    });
    public Task<T> Read<T>(Func<IDataSession, Task<T>> action, CancellationToken ct = default) => Execute(action, false, ct);
    public Task<T> Write<T>(Func<IDataSession, Task<T>> action, CancellationToken ct = default) => Execute(action, true, ct);
    private Task<T> Execute<T>(Func<IDataSession, Task<T>> action, bool write, CancellationToken ct) => Task.Run(async () =>
    {
        await Gate.WaitAsync(ct);
        try
        {
            await using var db = Context();
            await using var transaction = write ? await db.Database.BeginTransactionAsync(ct) : null;
            var result = await action(new DataSession(db, ct));
            if (write) { await db.SaveChangesAsync(ct); await transaction!.CommitAsync(ct); }
            return result;
        }
        catch (DbUpdateException ex) when (ex.InnerException is SqliteException { SqliteErrorCode: 19 }) { throw new RuleException("No se guardó el cambio porque hay datos duplicados o una relación inválida. Revisa la clave y el folio."); }
        catch (SqliteException ex) when (ex.SqliteErrorCode is 5 or 6) { throw new RuleException("La base de datos está ocupada. Espera un momento y vuelve a intentar."); }
        finally { Gate.Release(); }
    }, ct);
    private sealed class DataSession(PosDbContext db, CancellationToken ct) : IDataSession
    {
        public async Task<T?> Get<T>(int id) where T : Entity => await db.Set<T>().FindAsync([id], ct);
        public Task<List<T>> List<T>(Expression<Func<T, bool>>? filter = null, int skip = 0, int take = 5000) where T : Entity
        {
            IQueryable<T> query = db.Set<T>(); if (filter is not null) query = query.Where(filter);
            return query.OrderByDescending(x => x.Id).Skip(skip).Take(take).ToListAsync(ct);
        }
        public Task<int> Count<T>(Expression<Func<T, bool>>? filter = null) where T : Entity => filter is null ? db.Set<T>().CountAsync(ct) : db.Set<T>().CountAsync(filter, ct);
        public void Add<T>(T entity) where T : Entity => db.Add(entity);
        public void Remove<T>(T entity) where T : Entity => db.Remove(entity);
        public Task Flush() => db.SaveChangesAsync(ct);
    }
}
