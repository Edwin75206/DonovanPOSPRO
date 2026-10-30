using DonovanPOS.Domain;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Design;
namespace DonovanPOS.Infrastructure.Persistence;

public sealed class PosDbContext(DbContextOptions<PosDbContext> options) : DbContext(options)
{
    public DbSet<Product> Products => Set<Product>();
    public DbSet<Sale> Sales => Set<Sale>();
    public DbSet<SaleLine> SaleLines => Set<SaleLine>();
    protected override void OnModelCreating(ModelBuilder b)
    {
        foreach (var type in typeof(Entity).Assembly.GetTypes().Where(t => !t.IsAbstract && typeof(Entity).IsAssignableFrom(t)))
        {
            b.Entity(type).HasKey(nameof(Entity.Id));
            b.Entity(type).ToTable(type.Name);
        }
        b.Entity<Product>().HasIndex(x => x.Sku).IsUnique();
        b.Entity<Product>().HasIndex(x => x.Barcode).IsUnique();
        b.Entity<Product>().HasIndex(x => new { x.Active, x.Name });
        b.Entity<Product>().HasIndex(x => x.Category);
        b.Entity<Product>().ToTable(t => { t.HasCheckConstraint("CK_Product_Stock", "StockUnits >= 0 AND MinimumUnits >= 0 AND PackUnits > 0"); t.HasCheckConstraint("CK_Product_Price", "PriceCents >= 0 AND WholesaleCents >= 0 AND (CostCents IS NULL OR CostCents >= 0) AND TaxBasisPoints BETWEEN 0 AND 10000"); });
        b.Entity<User>().HasIndex(x => x.Username).IsUnique();
        b.Entity<Sequence>().HasIndex(x => x.Key).IsUnique();
        b.Entity<Sale>().HasIndex(x => x.RequestId).IsUnique();
        b.Entity<Sale>().HasIndex(x => x.Folio).IsUnique();
        b.Entity<Sale>().HasIndex(x => x.CreatedAt);
        b.Entity<Sale>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<Sale>().HasOne<Customer>().WithMany().HasForeignKey(x => x.CustomerId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<Sale>().HasOne<CashSession>().WithMany().HasForeignKey(x => x.CashSessionId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<Sale>().ToTable(t => t.HasCheckConstraint("CK_Sale_Total", "TotalCents >= 0 AND TaxCents >= 0 AND ChangeCents >= 0"));
        b.Entity<SaleLine>().HasOne<Sale>().WithMany().HasForeignKey(x => x.SaleId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<SaleLine>().HasOne<Product>().WithMany().HasForeignKey(x => x.ProductId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<SaleLine>().ToTable(t => t.HasCheckConstraint("CK_SaleLine_Quantity", "QuantityUnits > 0 AND ReturnedUnits >= 0 AND ReturnedUnits <= QuantityUnits AND RefundedCents >= 0 AND RefundedCents <= TotalCents"));
        b.Entity<PaymentRecord>().HasOne<Sale>().WithMany().HasForeignKey(x => x.SaleId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<PaymentRecord>().ToTable(t => t.HasCheckConstraint("CK_Payment_Amount", "AmountCents >= 0"));
        b.Entity<StockMovement>().HasOne<Product>().WithMany().HasForeignKey(x => x.ProductId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<StockMovement>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<StockMovement>().HasIndex(x => new { x.ProductId, x.CreatedAt });
        b.Entity<Purchase>().HasIndex(x => new { x.SupplierId, x.SupplierReference }).IsUnique();
        b.Entity<Purchase>().HasOne<Supplier>().WithMany().HasForeignKey(x => x.SupplierId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<Purchase>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<PurchaseLine>().HasOne<Purchase>().WithMany().HasForeignKey(x => x.PurchaseId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<PurchaseLine>().HasOne<Product>().WithMany().HasForeignKey(x => x.ProductId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<PurchaseLine>().ToTable(t => t.HasCheckConstraint("CK_PurchaseLine", "QuantityUnits > 0 AND CostCents >= 0"));
        b.Entity<CreditEntry>().HasOne<Sale>().WithMany().HasForeignKey(x => x.SaleId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<CreditEntry>().HasOne<Customer>().WithMany().HasForeignKey(x => x.CustomerId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<CreditEntry>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<CreditEntry>().HasIndex(x => new { x.CustomerId, x.SaleId });
        b.Entity<SaleReturn>().HasOne<Sale>().WithMany().HasForeignKey(x => x.SaleId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<SaleReturn>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<SaleReturn>().HasIndex(x => x.CreatedAt);
        b.Entity<ReturnLine>().HasOne<SaleReturn>().WithMany().HasForeignKey(x => x.SaleReturnId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<ReturnLine>().HasOne<SaleLine>().WithMany().HasForeignKey(x => x.SaleLineId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<Quote>().HasIndex(x => x.Folio).IsUnique();
        b.Entity<Quote>().HasOne<Sale>().WithMany().HasForeignKey(x => x.SaleId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<Quote>().HasOne<Customer>().WithMany().HasForeignKey(x => x.CustomerId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<Quote>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<QuoteLine>().HasOne<Quote>().WithMany().HasForeignKey(x => x.QuoteId).OnDelete(DeleteBehavior.Cascade);
        b.Entity<QuoteLine>().HasOne<Product>().WithMany().HasForeignKey(x => x.ProductId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<CashSession>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<CashSession>().HasIndex(x => x.Folio).IsUnique();
        b.Entity<CashMovement>().HasOne<CashSession>().WithMany().HasForeignKey(x => x.CashSessionId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<CashMovement>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<CashMovement>().HasIndex(x => new { x.CashSessionId, x.CreatedAt });
        b.Entity<AuditEvent>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<AuditEvent>().HasIndex(x => x.CreatedAt);
        b.Entity<HeldSale>().HasIndex(x => x.Folio).IsUnique();
        b.Entity<HeldSale>().HasOne<User>().WithMany().HasForeignKey(x => x.UserId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<HeldSale>().HasOne<Customer>().WithMany().HasForeignKey(x => x.CustomerId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<HeldSale>().HasOne<Sale>().WithMany().HasForeignKey(x => x.SaleId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<HeldSaleLine>().HasOne<HeldSale>().WithMany().HasForeignKey(x => x.HeldSaleId).OnDelete(DeleteBehavior.Cascade);
        b.Entity<HeldSaleLine>().HasOne<Product>().WithMany().HasForeignKey(x => x.ProductId).OnDelete(DeleteBehavior.Restrict);
        b.Entity<HeldSaleLine>().ToTable(t => t.HasCheckConstraint("CK_Held_Quantity", "QuantityUnits > 0 AND OriginalPriceCents >= 0"));
    }
}
public sealed class DesignTimeFactory : IDesignTimeDbContextFactory<PosDbContext>
{
    public PosDbContext CreateDbContext(string[] args) => new(new DbContextOptionsBuilder<PosDbContext>().UseSqlite("Data Source=design.db").Options);
}
