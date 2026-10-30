using DonovanPOS.Application;
using DonovanPOS.Infrastructure.Persistence;
using DonovanPOS.Infrastructure.Security;
using Microsoft.Extensions.DependencyInjection;
namespace DonovanPOS.Infrastructure;

public static class DependencyInjection
{
    public static IServiceCollection AddDonovan(this IServiceCollection services, string dataDirectory)
    {
        services.AddSingleton(new Database(dataDirectory));
        services.AddSingleton<IDataStore>(s => s.GetRequiredService<Database>());
        services.AddSingleton<UserSession>(); services.AddSingleton<IClock, MexicoClock>(); services.AddSingleton<IPasswordHasher, PasswordHasher>();
        services.AddSingleton<AuthService>(); services.AddSingleton<CatalogService>(); services.AddSingleton<SalesService>(); services.AddSingleton<ReturnService>();
        services.AddSingleton<CustomersService>(); services.AddSingleton<PurchasesService>(); services.AddSingleton<QuotesService>(); services.AddSingleton<CashService>(); services.AddSingleton<SettingsService>();
        services.AddSingleton<IBackupService, BackupService>(); services.AddSingleton<IDocumentService, Documents.DocumentService>(); services.AddSingleton<IReportingService, ReportingService>(); services.AddSingleton<ImportService>();
        return services;
    }
}
public sealed class MexicoClock : IClock
{
    public DateTime Now => TimeZoneInfo.ConvertTimeFromUtc(DateTime.UtcNow, TimeZoneInfo.FindSystemTimeZoneById("Central Standard Time (Mexico)"));
}
