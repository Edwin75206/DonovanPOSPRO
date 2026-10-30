using System.Linq.Expressions;
using DonovanPOS.Domain;
namespace DonovanPOS.Application;

public interface IDataSession
{
    Task<T?> Get<T>(int id) where T : Entity;
    Task<List<T>> List<T>(Expression<Func<T, bool>>? filter = null, int skip = 0, int take = 5000) where T : Entity;
    Task<int> Count<T>(Expression<Func<T, bool>>? filter = null) where T : Entity;
    void Add<T>(T entity) where T : Entity;
    void Remove<T>(T entity) where T : Entity;
    Task Flush();
}
public interface IDataStore
{
    Task<T> Read<T>(Func<IDataSession, Task<T>> action, CancellationToken ct = default);
    Task<T> Write<T>(Func<IDataSession, Task<T>> action, CancellationToken ct = default);
}
public interface IPasswordHasher { string Hash(string password); bool Verify(string password, string encoded); }
public interface IClock { DateTime Now { get; } }
public interface IBackupService { Task<string> Backup(string folder); Task Restore(string file); string DataDirectory { get; } }
public sealed record TableData(string Title, string[] Headers, List<string[]> Rows);
public sealed record DocumentLine(string Description, string Quantity, decimal Price, decimal Total);
public sealed record BusinessDocument(string Title, string Folio, DateTime Date, string Customer, IReadOnlyList<DocumentLine> Lines, decimal Total, decimal Tax, string Notes);
public interface IDocumentService
{
    Task Pdf(string path, StoreSettings store, BusinessDocument document, int? ticketWidth = null);
    Task Excel(string path, TableData table);
    Task<TableData> ReadExcel(string path);
    Task ReportPdf(string path, StoreSettings store, TableData table);
}
public sealed record SessionUser(int Id, string Username, string Name, Role Role);
public sealed class UserSession
{
    public SessionUser? Current { get; private set; }
    internal void SignIn(User user) => Current = new(user.Id, user.Username, user.Name, user.Role);
    public void SignOut() => Current = null;
    public bool Can(Permission permission) => Current is not null && (Current.Role == Role.Administrador || permission is Permission.Sell or Permission.Quote or Permission.Cash);
    public SessionUser Require(Permission permission)
    {
        Amount.Require(Current is not null, "Inicia sesión para continuar.");
        Amount.Require(Can(permission), "Esta acción requiere una cuenta de administrador.");
        return Current!;
    }
}
public sealed record ProductPage(List<Product> Items, int Total);
public sealed record SaleItem(int ProductId, decimal Quantity, bool Wholesale = false);
public sealed record PaymentInput(PayMethod Method, decimal Amount, string Reference = "");
public sealed record CheckoutRequest(string RequestId, IReadOnlyList<SaleItem> Items, IReadOnlyList<PaymentInput> Payments, decimal Tendered, decimal DiscountPercent = 0, int? CustomerId = null, DateTime? DueDate = null, int? QuoteId = null);
public sealed record PurchaseItem(int ProductId, decimal Quantity, decimal UnitCost, bool ByPack = false);
public sealed record ReturnItem(int SaleLineId, decimal Quantity);
public sealed record SaleDetail(Sale Sale, List<SaleLine> Lines, List<PaymentRecord> Payments, List<SaleReturn> Returns);
public sealed record QuoteDetail(Quote Quote, List<QuoteLine> Lines);
public sealed record CashOverview(CashSession? Session, long ExpectedCents, List<CashMovement> Movements, List<CashSession> History);
public sealed record CustomerBalance(Customer Customer, long BalanceCents);
public sealed record ChartPoint(string Label, decimal Value);
public sealed record DashboardData(long TodayCents, int TodayCount, long WeekCents, long MonthCents, long PreviousPeriodCents, long PeriodCents, long CashCents, long ReceivableCents, long? ProfitCents, List<ChartPoint> Trend, List<ChartPoint> TopProducts, List<ChartPoint> Categories, List<ChartPoint> Methods, List<Product> LowStock, List<Sale> Recent);
public interface IReportingService
{
    Task<DashboardData> Dashboard(DateTime from, DateTime to, CancellationToken ct = default);
    Task<TableData> Report(string kind, DateTime from, DateTime to, CancellationToken ct = default);
}
