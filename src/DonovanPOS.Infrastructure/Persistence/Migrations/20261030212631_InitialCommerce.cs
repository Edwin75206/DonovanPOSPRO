using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace DonovanPOS.Infrastructure.Persistence.Migrations
{
    /// <inheritdoc />
    public partial class InitialCommerce : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "Customer",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Phone = table.Column<string>(type: "TEXT", nullable: false),
                    Email = table.Column<string>(type: "TEXT", nullable: false),
                    Address = table.Column<string>(type: "TEXT", nullable: false),
                    CreditLimitCents = table.Column<long>(type: "INTEGER", nullable: false),
                    Active = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Customer", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Product",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Sku = table.Column<string>(type: "TEXT", nullable: false),
                    Barcode = table.Column<string>(type: "TEXT", nullable: true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Category = table.Column<string>(type: "TEXT", nullable: false),
                    Brand = table.Column<string>(type: "TEXT", nullable: false),
                    Size = table.Column<string>(type: "TEXT", nullable: false),
                    Unit = table.Column<int>(type: "INTEGER", nullable: false),
                    CostCents = table.Column<long>(type: "INTEGER", nullable: true),
                    PriceCents = table.Column<long>(type: "INTEGER", nullable: false),
                    WholesaleCents = table.Column<long>(type: "INTEGER", nullable: false),
                    StockUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    MinimumUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    TaxBasisPoints = table.Column<int>(type: "INTEGER", nullable: false),
                    PackName = table.Column<string>(type: "TEXT", nullable: false),
                    PackUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    Active = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Product", x => x.Id);
                    table.CheckConstraint("CK_Product_Price", "PriceCents >= 0 AND WholesaleCents >= 0 AND (CostCents IS NULL OR CostCents >= 0) AND TaxBasisPoints BETWEEN 0 AND 10000");
                    table.CheckConstraint("CK_Product_Stock", "StockUnits >= 0 AND MinimumUnits >= 0 AND PackUnits > 0");
                });

            migrationBuilder.CreateTable(
                name: "Sequence",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Key = table.Column<string>(type: "TEXT", nullable: false),
                    Value = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Sequence", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "StoreSettings",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    StoreName = table.Column<string>(type: "TEXT", nullable: false),
                    Address = table.Column<string>(type: "TEXT", nullable: false),
                    Phone = table.Column<string>(type: "TEXT", nullable: false),
                    Email = table.Column<string>(type: "TEXT", nullable: false),
                    TaxId = table.Column<string>(type: "TEXT", nullable: false),
                    LogoPath = table.Column<string>(type: "TEXT", nullable: false),
                    TicketFooter = table.Column<string>(type: "TEXT", nullable: false),
                    FolioPrefix = table.Column<string>(type: "TEXT", nullable: false),
                    DefaultTaxBasisPoints = table.Column<int>(type: "INTEGER", nullable: false),
                    TicketWidthMm = table.Column<int>(type: "INTEGER", nullable: false),
                    PrinterName = table.Column<string>(type: "TEXT", nullable: false),
                    BackupDirectory = table.Column<string>(type: "TEXT", nullable: false),
                    DarkMode = table.Column<bool>(type: "INTEGER", nullable: false),
                    ReducedMotion = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_StoreSettings", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Supplier",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Phone = table.Column<string>(type: "TEXT", nullable: false),
                    Email = table.Column<string>(type: "TEXT", nullable: false),
                    Address = table.Column<string>(type: "TEXT", nullable: false),
                    Active = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Supplier", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "User",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Username = table.Column<string>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    PasswordHash = table.Column<string>(type: "TEXT", nullable: false),
                    Role = table.Column<int>(type: "INTEGER", nullable: false),
                    Active = table.Column<bool>(type: "INTEGER", nullable: false),
                    FailedAttempts = table.Column<int>(type: "INTEGER", nullable: false),
                    LockedUntil = table.Column<DateTime>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_User", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "AuditEvent",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: true),
                    Action = table.Column<string>(type: "TEXT", nullable: false),
                    Reference = table.Column<string>(type: "TEXT", nullable: false),
                    Detail = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_AuditEvent", x => x.Id);
                    table.ForeignKey(
                        name: "FK_AuditEvent_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "CashSession",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Folio = table.Column<string>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    OpenedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    ClosedAt = table.Column<DateTime>(type: "TEXT", nullable: true),
                    OpeningCents = table.Column<long>(type: "INTEGER", nullable: false),
                    ExpectedCents = table.Column<long>(type: "INTEGER", nullable: true),
                    CountedCents = table.Column<long>(type: "INTEGER", nullable: true),
                    Notes = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_CashSession", x => x.Id);
                    table.ForeignKey(
                        name: "FK_CashSession_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "Purchase",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    SupplierId = table.Column<int>(type: "INTEGER", nullable: false),
                    Folio = table.Column<string>(type: "TEXT", nullable: false),
                    SupplierReference = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    TotalCents = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Purchase", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Purchase_Supplier_SupplierId",
                        column: x => x.SupplierId,
                        principalTable: "Supplier",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Purchase_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "StockMovement",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    ProductId = table.Column<int>(type: "INTEGER", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    Kind = table.Column<string>(type: "TEXT", nullable: false),
                    Reference = table.Column<string>(type: "TEXT", nullable: false),
                    Reason = table.Column<string>(type: "TEXT", nullable: false),
                    DeltaUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    BalanceUnits = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_StockMovement", x => x.Id);
                    table.ForeignKey(
                        name: "FK_StockMovement_Product_ProductId",
                        column: x => x.ProductId,
                        principalTable: "Product",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_StockMovement_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "CashMovement",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    CashSessionId = table.Column<int>(type: "INTEGER", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    AmountCents = table.Column<long>(type: "INTEGER", nullable: false),
                    Kind = table.Column<string>(type: "TEXT", nullable: false),
                    Reference = table.Column<string>(type: "TEXT", nullable: false),
                    Reason = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_CashMovement", x => x.Id);
                    table.ForeignKey(
                        name: "FK_CashMovement_CashSession_CashSessionId",
                        column: x => x.CashSessionId,
                        principalTable: "CashSession",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_CashMovement_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "Sale",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Folio = table.Column<string>(type: "TEXT", nullable: false),
                    RequestId = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    Cashier = table.Column<string>(type: "TEXT", nullable: false),
                    CustomerId = table.Column<int>(type: "INTEGER", nullable: true),
                    CustomerName = table.Column<string>(type: "TEXT", nullable: false),
                    CashSessionId = table.Column<int>(type: "INTEGER", nullable: false),
                    TotalCents = table.Column<long>(type: "INTEGER", nullable: false),
                    TaxCents = table.Column<long>(type: "INTEGER", nullable: false),
                    DiscountCents = table.Column<long>(type: "INTEGER", nullable: false),
                    ChangeCents = table.Column<long>(type: "INTEGER", nullable: false),
                    TenderedCents = table.Column<long>(type: "INTEGER", nullable: false),
                    Cancelled = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Sale", x => x.Id);
                    table.CheckConstraint("CK_Sale_Total", "TotalCents >= 0 AND TaxCents >= 0 AND ChangeCents >= 0");
                    table.ForeignKey(
                        name: "FK_Sale_CashSession_CashSessionId",
                        column: x => x.CashSessionId,
                        principalTable: "CashSession",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Sale_Customer_CustomerId",
                        column: x => x.CustomerId,
                        principalTable: "Customer",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Sale_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "PurchaseLine",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    PurchaseId = table.Column<int>(type: "INTEGER", nullable: false),
                    ProductId = table.Column<int>(type: "INTEGER", nullable: false),
                    QuantityUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    CostCents = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_PurchaseLine", x => x.Id);
                    table.CheckConstraint("CK_PurchaseLine", "QuantityUnits > 0 AND CostCents >= 0");
                    table.ForeignKey(
                        name: "FK_PurchaseLine_Product_ProductId",
                        column: x => x.ProductId,
                        principalTable: "Product",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_PurchaseLine_Purchase_PurchaseId",
                        column: x => x.PurchaseId,
                        principalTable: "Purchase",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "CreditEntry",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    CustomerId = table.Column<int>(type: "INTEGER", nullable: false),
                    SaleId = table.Column<int>(type: "INTEGER", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    DueDate = table.Column<DateTime>(type: "TEXT", nullable: true),
                    AmountCents = table.Column<long>(type: "INTEGER", nullable: false),
                    Method = table.Column<int>(type: "INTEGER", nullable: false),
                    Kind = table.Column<string>(type: "TEXT", nullable: false),
                    Reference = table.Column<string>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_CreditEntry", x => x.Id);
                    table.ForeignKey(
                        name: "FK_CreditEntry_Customer_CustomerId",
                        column: x => x.CustomerId,
                        principalTable: "Customer",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_CreditEntry_Sale_SaleId",
                        column: x => x.SaleId,
                        principalTable: "Sale",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_CreditEntry_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "PaymentRecord",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    SaleId = table.Column<int>(type: "INTEGER", nullable: false),
                    Method = table.Column<int>(type: "INTEGER", nullable: false),
                    AmountCents = table.Column<long>(type: "INTEGER", nullable: false),
                    Reference = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_PaymentRecord", x => x.Id);
                    table.CheckConstraint("CK_Payment_Amount", "AmountCents >= 0");
                    table.ForeignKey(
                        name: "FK_PaymentRecord_Sale_SaleId",
                        column: x => x.SaleId,
                        principalTable: "Sale",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "Quote",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Folio = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    ValidUntil = table.Column<DateTime>(type: "TEXT", nullable: false),
                    CustomerId = table.Column<int>(type: "INTEGER", nullable: true),
                    CustomerName = table.Column<string>(type: "TEXT", nullable: false),
                    Notes = table.Column<string>(type: "TEXT", nullable: false),
                    TotalCents = table.Column<long>(type: "INTEGER", nullable: false),
                    TaxCents = table.Column<long>(type: "INTEGER", nullable: false),
                    SaleId = table.Column<int>(type: "INTEGER", nullable: true),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Quote", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Quote_Customer_CustomerId",
                        column: x => x.CustomerId,
                        principalTable: "Customer",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Quote_Sale_SaleId",
                        column: x => x.SaleId,
                        principalTable: "Sale",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Quote_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "SaleLine",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    SaleId = table.Column<int>(type: "INTEGER", nullable: false),
                    ProductId = table.Column<int>(type: "INTEGER", nullable: false),
                    Sku = table.Column<string>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Category = table.Column<string>(type: "TEXT", nullable: false),
                    Unit = table.Column<int>(type: "INTEGER", nullable: false),
                    QuantityUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    PriceCents = table.Column<long>(type: "INTEGER", nullable: false),
                    CostCents = table.Column<long>(type: "INTEGER", nullable: true),
                    TaxBasisPoints = table.Column<int>(type: "INTEGER", nullable: false),
                    TotalCents = table.Column<long>(type: "INTEGER", nullable: false),
                    TaxCents = table.Column<long>(type: "INTEGER", nullable: false),
                    ReturnedUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    RefundedCents = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_SaleLine", x => x.Id);
                    table.CheckConstraint("CK_SaleLine_Quantity", "QuantityUnits > 0 AND ReturnedUnits >= 0 AND ReturnedUnits <= QuantityUnits AND RefundedCents >= 0 AND RefundedCents <= TotalCents");
                    table.ForeignKey(
                        name: "FK_SaleLine_Product_ProductId",
                        column: x => x.ProductId,
                        principalTable: "Product",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_SaleLine_Sale_SaleId",
                        column: x => x.SaleId,
                        principalTable: "Sale",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "SaleReturn",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    SaleId = table.Column<int>(type: "INTEGER", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    Folio = table.Column<string>(type: "TEXT", nullable: false),
                    Reason = table.Column<string>(type: "TEXT", nullable: false),
                    TotalCents = table.Column<long>(type: "INTEGER", nullable: false),
                    CreditReductionCents = table.Column<long>(type: "INTEGER", nullable: false),
                    RefundMethod = table.Column<int>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_SaleReturn", x => x.Id);
                    table.ForeignKey(
                        name: "FK_SaleReturn_Sale_SaleId",
                        column: x => x.SaleId,
                        principalTable: "Sale",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_SaleReturn_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "QuoteLine",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    QuoteId = table.Column<int>(type: "INTEGER", nullable: false),
                    ProductId = table.Column<int>(type: "INTEGER", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Unit = table.Column<int>(type: "INTEGER", nullable: false),
                    QuantityUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    PriceCents = table.Column<long>(type: "INTEGER", nullable: false),
                    TaxBasisPoints = table.Column<int>(type: "INTEGER", nullable: false),
                    TotalCents = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_QuoteLine", x => x.Id);
                    table.ForeignKey(
                        name: "FK_QuoteLine_Product_ProductId",
                        column: x => x.ProductId,
                        principalTable: "Product",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_QuoteLine_Quote_QuoteId",
                        column: x => x.QuoteId,
                        principalTable: "Quote",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "ReturnLine",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    SaleReturnId = table.Column<int>(type: "INTEGER", nullable: false),
                    SaleLineId = table.Column<int>(type: "INTEGER", nullable: false),
                    QuantityUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    AmountCents = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_ReturnLine", x => x.Id);
                    table.ForeignKey(
                        name: "FK_ReturnLine_SaleLine_SaleLineId",
                        column: x => x.SaleLineId,
                        principalTable: "SaleLine",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_ReturnLine_SaleReturn_SaleReturnId",
                        column: x => x.SaleReturnId,
                        principalTable: "SaleReturn",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateIndex(
                name: "IX_AuditEvent_CreatedAt",
                table: "AuditEvent",
                column: "CreatedAt");

            migrationBuilder.CreateIndex(
                name: "IX_AuditEvent_UserId",
                table: "AuditEvent",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_CashMovement_CashSessionId_CreatedAt",
                table: "CashMovement",
                columns: new[] { "CashSessionId", "CreatedAt" });

            migrationBuilder.CreateIndex(
                name: "IX_CashMovement_UserId",
                table: "CashMovement",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_CashSession_Folio",
                table: "CashSession",
                column: "Folio",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_CashSession_UserId",
                table: "CashSession",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_CreditEntry_CustomerId_SaleId",
                table: "CreditEntry",
                columns: new[] { "CustomerId", "SaleId" });

            migrationBuilder.CreateIndex(
                name: "IX_CreditEntry_SaleId",
                table: "CreditEntry",
                column: "SaleId");

            migrationBuilder.CreateIndex(
                name: "IX_CreditEntry_UserId",
                table: "CreditEntry",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_PaymentRecord_SaleId",
                table: "PaymentRecord",
                column: "SaleId");

            migrationBuilder.CreateIndex(
                name: "IX_Product_Active_Name",
                table: "Product",
                columns: new[] { "Active", "Name" });

            migrationBuilder.CreateIndex(
                name: "IX_Product_Barcode",
                table: "Product",
                column: "Barcode",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Product_Category",
                table: "Product",
                column: "Category");

            migrationBuilder.CreateIndex(
                name: "IX_Product_Sku",
                table: "Product",
                column: "Sku",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Purchase_SupplierId_SupplierReference",
                table: "Purchase",
                columns: new[] { "SupplierId", "SupplierReference" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Purchase_UserId",
                table: "Purchase",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_PurchaseLine_ProductId",
                table: "PurchaseLine",
                column: "ProductId");

            migrationBuilder.CreateIndex(
                name: "IX_PurchaseLine_PurchaseId",
                table: "PurchaseLine",
                column: "PurchaseId");

            migrationBuilder.CreateIndex(
                name: "IX_Quote_CustomerId",
                table: "Quote",
                column: "CustomerId");

            migrationBuilder.CreateIndex(
                name: "IX_Quote_Folio",
                table: "Quote",
                column: "Folio",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Quote_SaleId",
                table: "Quote",
                column: "SaleId");

            migrationBuilder.CreateIndex(
                name: "IX_Quote_UserId",
                table: "Quote",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_QuoteLine_ProductId",
                table: "QuoteLine",
                column: "ProductId");

            migrationBuilder.CreateIndex(
                name: "IX_QuoteLine_QuoteId",
                table: "QuoteLine",
                column: "QuoteId");

            migrationBuilder.CreateIndex(
                name: "IX_ReturnLine_SaleLineId",
                table: "ReturnLine",
                column: "SaleLineId");

            migrationBuilder.CreateIndex(
                name: "IX_ReturnLine_SaleReturnId",
                table: "ReturnLine",
                column: "SaleReturnId");

            migrationBuilder.CreateIndex(
                name: "IX_Sale_CashSessionId",
                table: "Sale",
                column: "CashSessionId");

            migrationBuilder.CreateIndex(
                name: "IX_Sale_CreatedAt",
                table: "Sale",
                column: "CreatedAt");

            migrationBuilder.CreateIndex(
                name: "IX_Sale_CustomerId",
                table: "Sale",
                column: "CustomerId");

            migrationBuilder.CreateIndex(
                name: "IX_Sale_Folio",
                table: "Sale",
                column: "Folio",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Sale_RequestId",
                table: "Sale",
                column: "RequestId",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Sale_UserId",
                table: "Sale",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_SaleLine_ProductId",
                table: "SaleLine",
                column: "ProductId");

            migrationBuilder.CreateIndex(
                name: "IX_SaleLine_SaleId",
                table: "SaleLine",
                column: "SaleId");

            migrationBuilder.CreateIndex(
                name: "IX_SaleReturn_CreatedAt",
                table: "SaleReturn",
                column: "CreatedAt");

            migrationBuilder.CreateIndex(
                name: "IX_SaleReturn_SaleId",
                table: "SaleReturn",
                column: "SaleId");

            migrationBuilder.CreateIndex(
                name: "IX_SaleReturn_UserId",
                table: "SaleReturn",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_Sequence_Key",
                table: "Sequence",
                column: "Key",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_StockMovement_ProductId_CreatedAt",
                table: "StockMovement",
                columns: new[] { "ProductId", "CreatedAt" });

            migrationBuilder.CreateIndex(
                name: "IX_StockMovement_UserId",
                table: "StockMovement",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_User_Username",
                table: "User",
                column: "Username",
                unique: true);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "AuditEvent");

            migrationBuilder.DropTable(
                name: "CashMovement");

            migrationBuilder.DropTable(
                name: "CreditEntry");

            migrationBuilder.DropTable(
                name: "PaymentRecord");

            migrationBuilder.DropTable(
                name: "PurchaseLine");

            migrationBuilder.DropTable(
                name: "QuoteLine");

            migrationBuilder.DropTable(
                name: "ReturnLine");

            migrationBuilder.DropTable(
                name: "Sequence");

            migrationBuilder.DropTable(
                name: "StockMovement");

            migrationBuilder.DropTable(
                name: "StoreSettings");

            migrationBuilder.DropTable(
                name: "Purchase");

            migrationBuilder.DropTable(
                name: "Quote");

            migrationBuilder.DropTable(
                name: "SaleLine");

            migrationBuilder.DropTable(
                name: "SaleReturn");

            migrationBuilder.DropTable(
                name: "Supplier");

            migrationBuilder.DropTable(
                name: "Product");

            migrationBuilder.DropTable(
                name: "Sale");

            migrationBuilder.DropTable(
                name: "CashSession");

            migrationBuilder.DropTable(
                name: "Customer");

            migrationBuilder.DropTable(
                name: "User");
        }
    }
}
