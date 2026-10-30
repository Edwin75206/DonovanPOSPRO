namespace DonovanPOS.Domain;

public sealed class User : Entity
{
    public string Username { get; set; } = "";
    public string Name { get; set; } = "";
    public string PasswordHash { get; set; } = "";
    public Role Role { get; set; }
    public bool Active { get; set; } = true;
    public int FailedAttempts { get; set; }
    public DateTime? LockedUntil { get; set; }
}
public sealed class CashSession : Entity
{
    public string Folio { get; set; } = "";
    public int UserId { get; set; }
    public DateTime OpenedAt { get; set; }
    public DateTime? ClosedAt { get; set; }
    public long OpeningCents { get; set; }
    public long? ExpectedCents { get; set; }
    public long? CountedCents { get; set; }
    public string Notes { get; set; } = "";
}
public sealed class CashMovement : Entity
{
    public int CashSessionId { get; set; }
    public int UserId { get; set; }
    public DateTime CreatedAt { get; set; }
    public long AmountCents { get; set; }
    public string Kind { get; set; } = "";
    public string Reference { get; set; } = "";
    public string Reason { get; set; } = "";
}
public sealed class StoreSettings : Entity
{
    public string StoreName { get; set; } = "Mi ferretería";
    public string Address { get; set; } = "";
    public string Phone { get; set; } = "";
    public string Email { get; set; } = "";
    public string TaxId { get; set; } = "";
    public string LogoPath { get; set; } = "";
    public string TicketFooter { get; set; } = "Gracias por su compra";
    public string FolioPrefix { get; set; } = "V";
    public int DefaultTaxBasisPoints { get; set; } = 1600;
    public int TicketWidthMm { get; set; } = 80;
    public string PrinterName { get; set; } = "";
    public string BackupDirectory { get; set; } = "";
    public bool DarkMode { get; set; }
    public bool ReducedMotion { get; set; }
}
public sealed class Sequence : Entity { public string Key { get; set; } = ""; public long Value { get; set; } }
public sealed class AuditEvent : Entity
{
    public DateTime CreatedAt { get; set; }
    public int? UserId { get; set; }
    public string Action { get; set; } = "";
    public string Reference { get; set; } = "";
    public string Detail { get; set; } = "";
}
