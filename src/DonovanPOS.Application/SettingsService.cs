using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public sealed class SettingsService(IDataStore store, UserSession session, IClock clock)
{
    public Task<StoreSettings> Get() => store.Read(async db => (await db.List<StoreSettings>()).SingleOrDefault() ?? new StoreSettings());
    public Task<bool> Save(StoreSettings input)
    {
        var actor = session.Require(Permission.Settings);
        input.StoreName = Amount.Text(input.StoreName, "Comercio", 150);
        Amount.Require(input.TicketWidthMm is 58 or 80, "El ancho del ticket debe ser de 58 u 80 mm.");
        Amount.Require(input.DefaultTaxBasisPoints is >= 0 and <= 10000, "Revisa el porcentaje de impuesto.");
        Amount.Require(input.FolioPrefix.Length is >= 1 and <= 8 && input.FolioPrefix.All(char.IsLetterOrDigit), "Usa entre 1 y 8 letras o números para el prefijo.");
        return store.Write(async db =>
        {
            var s = (await db.List<StoreSettings>()).Single();
            s.StoreName = input.StoreName; s.Address = input.Address; s.Phone = input.Phone; s.Email = input.Email; s.TaxId = input.TaxId; s.LogoPath = input.LogoPath;
            s.TicketFooter = input.TicketFooter; s.TicketWidthMm = input.TicketWidthMm; s.PrinterName = input.PrinterName; s.DefaultTaxBasisPoints = input.DefaultTaxBasisPoints;
            s.FolioPrefix = input.FolioPrefix.ToUpperInvariant(); s.BackupDirectory = input.BackupDirectory; s.DarkMode = input.DarkMode; s.ReducedMotion = input.ReducedMotion;
            Operations.Audit(db, clock, actor.Id, "Guardar configuración", "Comercio"); return true;
        });
    }
    public Task<List<AuditEvent>> Audit() { session.Require(Permission.Settings); return store.Read(db => db.List<AuditEvent>(take: 1000)); }
}
