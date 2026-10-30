using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace DonovanPOS.Infrastructure.Persistence.Migrations
{
    /// <inheritdoc />
    public partial class ProductSearchFields : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<string>(
                name: "Aliases",
                table: "Product",
                type: "TEXT",
                nullable: false,
                defaultValue: "");
            migrationBuilder.Sql("""
                CREATE VIRTUAL TABLE ProductSearch USING fts5(Sku, Barcode, Name, Category, Brand, Size, Aliases, content='Product', content_rowid='Id', tokenize='unicode61 remove_diacritics 2');
                CREATE TRIGGER product_search_insert AFTER INSERT ON Product BEGIN
                  INSERT INTO ProductSearch(rowid,Sku,Barcode,Name,Category,Brand,Size,Aliases) VALUES(new.Id,new.Sku,new.Barcode,new.Name,new.Category,new.Brand,new.Size,new.Aliases);
                END;
                CREATE TRIGGER product_search_delete AFTER DELETE ON Product BEGIN
                  INSERT INTO ProductSearch(ProductSearch,rowid,Sku,Barcode,Name,Category,Brand,Size,Aliases) VALUES('delete',old.Id,old.Sku,old.Barcode,old.Name,old.Category,old.Brand,old.Size,old.Aliases);
                END;
                CREATE TRIGGER product_search_update AFTER UPDATE OF Sku,Barcode,Name,Category,Brand,Size,Aliases ON Product BEGIN
                  INSERT INTO ProductSearch(ProductSearch,rowid,Sku,Barcode,Name,Category,Brand,Size,Aliases) VALUES('delete',old.Id,old.Sku,old.Barcode,old.Name,old.Category,old.Brand,old.Size,old.Aliases);
                  INSERT INTO ProductSearch(rowid,Sku,Barcode,Name,Category,Brand,Size,Aliases) VALUES(new.Id,new.Sku,new.Barcode,new.Name,new.Category,new.Brand,new.Size,new.Aliases);
                END;
                INSERT INTO ProductSearch(ProductSearch) VALUES('rebuild');
                """);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.Sql("DROP TRIGGER product_search_insert; DROP TRIGGER product_search_delete; DROP TRIGGER product_search_update; DROP TABLE ProductSearch;");
            migrationBuilder.DropColumn(
                name: "Aliases",
                table: "Product");
        }
    }
}
