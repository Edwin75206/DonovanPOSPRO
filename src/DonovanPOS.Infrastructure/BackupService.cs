using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Infrastructure.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
namespace DonovanPOS.Infrastructure;

public sealed class BackupService(Database database, UserSession session) : IBackupService
{
    public string DataDirectory => database.DirectoryPath;
    public async Task<string> Backup(string folder)
    {
        session.Require(Permission.Settings); Directory.CreateDirectory(folder);
        var destination = Path.Combine(folder, $"DonovanPOS-{DateTime.Now:yyyyMMdd-HHmmss}-{Guid.NewGuid().ToString("N")[..6]}.db");
        await database.Gate.WaitAsync();
        try { await Task.Run(() => CopyDatabase(database.FilePath, destination)); return destination; }
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
            }
            var safety = Path.Combine(database.DirectoryPath, $"antes-de-restaurar-{DateTime.Now:yyyyMMdd-HHmmss}-{Guid.NewGuid().ToString("N")[..6]}.db");
            CopyDatabase(database.FilePath, safety);
            using (var conn = new SqliteConnection($"Data Source={database.FilePath};Pooling=False"))
            {
                conn.Open(); using var command = conn.CreateCommand(); command.CommandText = "PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;"; command.ExecuteNonQuery();
            }
            File.Move(staging, database.FilePath, true);
            session.SignOut();
        }
        finally { if (File.Exists(staging)) File.Delete(staging); database.Gate.Release(); }
    }
    private static void CopyDatabase(string source, string destination)
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
