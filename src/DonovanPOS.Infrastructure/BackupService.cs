using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Infrastructure.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
using System.Security.Cryptography;
namespace DonovanPOS.Infrastructure;

public sealed class BackupService(Database database, UserSession session, IClock clock) : IBackupService
{
    private readonly SemaphoreSlim backupGate = new(1, 1);
    public string DataDirectory => database.DirectoryPath;
    public async Task<string> Backup(string folder)
    {
        session.Require(Permission.Settings); await backupGate.WaitAsync();
        try { return await BackupCore(folder, "Manual"); } finally { backupGate.Release(); }
    }
    public Task<List<BackupRecord>> History() { session.Require(Permission.Settings); return database.Read(db => db.List<BackupRecord>(take: 100)); }
    public async Task<string?> BackupIfDue()
    {
        await backupGate.WaitAsync();
        try
        {
            var config = await database.Read(async db => (await db.List<StoreSettings>()).SingleOrDefault());
            if (config is null || !config.AutomaticBackups || string.IsNullOrWhiteSpace(config.BackupDirectory)) return null;
            var last = await database.Read(async db => (await db.List<BackupRecord>(x => x.Successful, take: 1)).SingleOrDefault());
            if (last is not null && clock.Now - last.CreatedAt < TimeSpan.FromHours(config.BackupIntervalHours)) return null;
            return await BackupCore(config.BackupDirectory, "Automático");
        }
        finally { backupGate.Release(); }
    }
    private async Task<string> BackupCore(string folder, string kind)
    {
        Amount.Require(!string.IsNullOrWhiteSpace(folder), "Selecciona una carpeta de respaldos.");
        var destination = Path.Combine(folder, $"DonovanPOS-{DateTime.Now:yyyyMMdd-HHmmss}-{Guid.NewGuid().ToString("N")[..6]}.db");
        var partial = destination + ".partial";
        await database.Gate.WaitAsync();
        try
        {
            Directory.CreateDirectory(folder);
            await Task.Run(() => { CopyDatabase(database.FilePath, partial); Validate(partial); }); File.Move(partial, destination);
            await using var db = database.Context();
            db.Add(new BackupRecord { CreatedAt = clock.Now, FilePath = destination, Kind = kind, Successful = true, FileBytes = new FileInfo(destination).Length, Sha256 = Convert.ToHexString(SHA256.HashData(await File.ReadAllBytesAsync(destination))), Result = "Integridad y relaciones verificadas" });
            db.Add(new AuditEvent { CreatedAt = clock.Now, UserId = kind == "Manual" ? session.Current?.Id : null, Action = "Crear respaldo " + kind.ToLowerInvariant(), Reference = Path.GetFileName(destination) }); await db.SaveChangesAsync();
            return destination;
        }
        catch (Exception ex)
        {
            if (File.Exists(partial)) File.Delete(partial);
            await using var db = database.Context(); db.Add(new BackupRecord { CreatedAt = clock.Now, FilePath = destination, Kind = kind, Successful = false, Result = ex is RuleException ? ex.Message : "No se pudo escribir o validar la copia. Revisa la carpeta y el espacio disponible." }); await db.SaveChangesAsync(); throw;
        }
        finally { database.Gate.Release(); }
    }
    public async Task Restore(string file)
    {
        session.Require(Permission.Settings);
        Amount.Require(!Path.GetFullPath(file).Equals(database.FilePath, StringComparison.OrdinalIgnoreCase), "Selecciona un respaldo diferente de la base actual.");
        Amount.Require(File.Exists(file), "No se encontró el respaldo seleccionado.");
        await database.Gate.WaitAsync();
        var staging = Path.Combine(database.DirectoryPath, "restore-" + Guid.NewGuid().ToString("N") + ".db");
        try
        {
            await Task.Run(() => { Validate(file); CopyDatabase(file, staging); Validate(staging); });
            await using (var staged = new PosDbContext(new DbContextOptionsBuilder<PosDbContext>().UseSqlite($"Data Source={staging};Pooling=False").Options))
            {
                var applied = (await staged.Database.GetAppliedMigrationsAsync()).ToHashSet();
                var known = staged.Database.GetMigrations().ToHashSet();
                Amount.Require(applied.Count > 0 && applied.IsSubsetOf(known), "El respaldo corresponde a otra versión. Actualiza el programa antes de restaurarlo.");
                await staged.Database.MigrateAsync();
                await staged.Database.ExecuteSqlRawAsync("CREATE UNIQUE INDEX IF NOT EXISTS IX_CashSession_OneOpen ON CashSession((1)) WHERE ClosedAt IS NULL;");
                staged.Add(new AuditEvent { CreatedAt = clock.Now, Action = "Restaurar respaldo", Reference = Path.GetFileName(file), Detail = "Integridad validada; copia previa conservada en la carpeta de datos." }); await staged.SaveChangesAsync();
            }
            var safety = Path.Combine(database.DirectoryPath, $"antes-de-restaurar-{DateTime.Now:yyyyMMdd-HHmmss}-{Guid.NewGuid().ToString("N")[..6]}.db");
            CopyDatabase(database.FilePath, safety);
            Validate(safety); Validate(staging);
            using (var conn = new SqliteConnection($"Data Source={database.FilePath};Pooling=False"))
            {
                conn.Open(); using var command = conn.CreateCommand(); command.CommandText = "PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;"; command.ExecuteNonQuery();
            }
            File.Move(staging, database.FilePath, true); session.SignOut();
            using (var restored = new SqliteConnection($"Data Source={database.FilePath};Pooling=False")) { restored.Open(); using var command = restored.CreateCommand(); command.CommandText = "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;"; command.ExecuteNonQuery(); }
        }
        finally { if (File.Exists(staging)) File.Delete(staging); database.Gate.Release(); }
    }
    internal static void CopyDatabase(string source, string destination)
    {
        using var from = new SqliteConnection(new SqliteConnectionStringBuilder { DataSource = source, Mode = SqliteOpenMode.ReadOnly, Pooling = false }.ToString());
        using var to = new SqliteConnection(new SqliteConnectionStringBuilder { DataSource = destination, Pooling = false }.ToString());
        from.Open(); to.Open(); from.BackupDatabase(to);
    }
    private static void Validate(string path)
    {
        using var conn = new SqliteConnection(new SqliteConnectionStringBuilder { DataSource = path, Mode = SqliteOpenMode.ReadOnly, Pooling = false }.ToString()); conn.Open();
        using var cmd = conn.CreateCommand(); cmd.CommandText = "PRAGMA integrity_check;";
        Amount.Require(Convert.ToString(cmd.ExecuteScalar()) == "ok", "El respaldo está dañado; no se reemplazó la base actual.");
        cmd.CommandText = "PRAGMA foreign_key_check;";
        using (var reader = cmd.ExecuteReader()) Amount.Require(!reader.Read(), "El respaldo tiene relaciones inválidas.");
        cmd.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('Sale','Product','User','__EFMigrationsHistory');";
        Amount.Require(Convert.ToInt32(cmd.ExecuteScalar()) == 4, "Este archivo no es un respaldo de Donovan POS.");
    }
}
