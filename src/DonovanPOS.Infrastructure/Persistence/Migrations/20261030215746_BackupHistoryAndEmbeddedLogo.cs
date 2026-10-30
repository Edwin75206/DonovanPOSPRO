using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace DonovanPOS.Infrastructure.Persistence.Migrations
{
    /// <inheritdoc />
    public partial class BackupHistoryAndEmbeddedLogo : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<bool>(
                name: "AutomaticBackups",
                table: "StoreSettings",
                type: "INTEGER",
                nullable: false,
                defaultValue: false);

            migrationBuilder.AddColumn<int>(
                name: "BackupIntervalHours",
                table: "StoreSettings",
                type: "INTEGER",
                nullable: false,
                defaultValue: 0);

            migrationBuilder.AddColumn<byte[]>(
                name: "LogoData",
                table: "StoreSettings",
                type: "BLOB",
                nullable: true);

            migrationBuilder.CreateTable(
                name: "BackupRecord",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    FilePath = table.Column<string>(type: "TEXT", nullable: false),
                    Kind = table.Column<string>(type: "TEXT", nullable: false),
                    Successful = table.Column<bool>(type: "INTEGER", nullable: false),
                    FileBytes = table.Column<long>(type: "INTEGER", nullable: false),
                    Sha256 = table.Column<string>(type: "TEXT", nullable: false),
                    Result = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_BackupRecord", x => x.Id);
                });
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "BackupRecord");

            migrationBuilder.DropColumn(
                name: "AutomaticBackups",
                table: "StoreSettings");

            migrationBuilder.DropColumn(
                name: "BackupIntervalHours",
                table: "StoreSettings");

            migrationBuilder.DropColumn(
                name: "LogoData",
                table: "StoreSettings");
        }
    }
}
