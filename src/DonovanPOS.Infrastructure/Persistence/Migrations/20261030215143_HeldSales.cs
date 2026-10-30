using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace DonovanPOS.Infrastructure.Persistence.Migrations
{
    /// <inheritdoc />
    public partial class HeldSales : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "HeldSale",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Folio = table.Column<string>(type: "TEXT", nullable: false),
                    Label = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    UserId = table.Column<int>(type: "INTEGER", nullable: false),
                    CustomerId = table.Column<int>(type: "INTEGER", nullable: true),
                    SaleId = table.Column<int>(type: "INTEGER", nullable: true),
                    DiscountParts = table.Column<int>(type: "INTEGER", nullable: false),
                    Discarded = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_HeldSale", x => x.Id);
                    table.ForeignKey(
                        name: "FK_HeldSale_Customer_CustomerId",
                        column: x => x.CustomerId,
                        principalTable: "Customer",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_HeldSale_Sale_SaleId",
                        column: x => x.SaleId,
                        principalTable: "Sale",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_HeldSale_User_UserId",
                        column: x => x.UserId,
                        principalTable: "User",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "HeldSaleLine",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    HeldSaleId = table.Column<int>(type: "INTEGER", nullable: false),
                    ProductId = table.Column<int>(type: "INTEGER", nullable: false),
                    QuantityUnits = table.Column<long>(type: "INTEGER", nullable: false),
                    Wholesale = table.Column<bool>(type: "INTEGER", nullable: false),
                    OriginalPriceCents = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_HeldSaleLine", x => x.Id);
                    table.CheckConstraint("CK_Held_Quantity", "QuantityUnits > 0 AND OriginalPriceCents >= 0");
                    table.ForeignKey(
                        name: "FK_HeldSaleLine_HeldSale_HeldSaleId",
                        column: x => x.HeldSaleId,
                        principalTable: "HeldSale",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_HeldSaleLine_Product_ProductId",
                        column: x => x.ProductId,
                        principalTable: "Product",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateIndex(
                name: "IX_HeldSale_CustomerId",
                table: "HeldSale",
                column: "CustomerId");

            migrationBuilder.CreateIndex(
                name: "IX_HeldSale_Folio",
                table: "HeldSale",
                column: "Folio",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_HeldSale_SaleId",
                table: "HeldSale",
                column: "SaleId");

            migrationBuilder.CreateIndex(
                name: "IX_HeldSale_UserId",
                table: "HeldSale",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_HeldSaleLine_HeldSaleId",
                table: "HeldSaleLine",
                column: "HeldSaleId");

            migrationBuilder.CreateIndex(
                name: "IX_HeldSaleLine_ProductId",
                table: "HeldSaleLine",
                column: "ProductId");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "HeldSaleLine");

            migrationBuilder.DropTable(
                name: "HeldSale");
        }
    }
}
