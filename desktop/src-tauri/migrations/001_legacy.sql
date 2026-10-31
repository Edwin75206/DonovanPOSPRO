-- Version 1.0.0 schema baseline. No business or user records.

CREATE TABLE "__EFMigrationsLock" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK___EFMigrationsLock" PRIMARY KEY,
    "Timestamp" TEXT NOT NULL
);

CREATE TABLE "__EFMigrationsHistory" (
    "MigrationId" TEXT NOT NULL CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY,
    "ProductVersion" TEXT NOT NULL
);

CREATE TABLE "Customer" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Customer" PRIMARY KEY AUTOINCREMENT,
    "Name" TEXT NOT NULL,
    "Phone" TEXT NOT NULL,
    "Email" TEXT NOT NULL,
    "Address" TEXT NOT NULL,
    "CreditLimitCents" INTEGER NOT NULL,
    "Active" INTEGER NOT NULL
);

CREATE TABLE "Product" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Product" PRIMARY KEY AUTOINCREMENT,
    "Sku" TEXT NOT NULL,
    "Barcode" TEXT NULL,
    "Name" TEXT NOT NULL,
    "Category" TEXT NOT NULL,
    "Brand" TEXT NOT NULL,
    "Size" TEXT NOT NULL,
    "Unit" INTEGER NOT NULL,
    "CostCents" INTEGER NULL,
    "PriceCents" INTEGER NOT NULL,
    "WholesaleCents" INTEGER NOT NULL,
    "StockUnits" INTEGER NOT NULL,
    "MinimumUnits" INTEGER NOT NULL,
    "TaxBasisPoints" INTEGER NOT NULL,
    "PackName" TEXT NOT NULL,
    "PackUnits" INTEGER NOT NULL,
    "Active" INTEGER NOT NULL, "Aliases" TEXT NOT NULL DEFAULT '',
    CONSTRAINT "CK_Product_Price" CHECK (PriceCents >= 0 AND WholesaleCents >= 0 AND (CostCents IS NULL OR CostCents >= 0) AND TaxBasisPoints BETWEEN 0 AND 10000),
    CONSTRAINT "CK_Product_Stock" CHECK (StockUnits >= 0 AND MinimumUnits >= 0 AND PackUnits > 0)
);

CREATE TABLE "Sequence" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Sequence" PRIMARY KEY AUTOINCREMENT,
    "Key" TEXT NOT NULL,
    "Value" INTEGER NOT NULL
);

CREATE TABLE "StoreSettings" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_StoreSettings" PRIMARY KEY AUTOINCREMENT,
    "StoreName" TEXT NOT NULL,
    "Address" TEXT NOT NULL,
    "Phone" TEXT NOT NULL,
    "Email" TEXT NOT NULL,
    "TaxId" TEXT NOT NULL,
    "LogoPath" TEXT NOT NULL,
    "TicketFooter" TEXT NOT NULL,
    "FolioPrefix" TEXT NOT NULL,
    "DefaultTaxBasisPoints" INTEGER NOT NULL,
    "TicketWidthMm" INTEGER NOT NULL,
    "PrinterName" TEXT NOT NULL,
    "BackupDirectory" TEXT NOT NULL,
    "DarkMode" INTEGER NOT NULL,
    "ReducedMotion" INTEGER NOT NULL
, "AutomaticBackups" INTEGER NOT NULL DEFAULT 0, "BackupIntervalHours" INTEGER NOT NULL DEFAULT 0, "LogoData" BLOB NULL);

CREATE TABLE "Supplier" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Supplier" PRIMARY KEY AUTOINCREMENT,
    "Name" TEXT NOT NULL,
    "Phone" TEXT NOT NULL,
    "Email" TEXT NOT NULL,
    "Address" TEXT NOT NULL,
    "Active" INTEGER NOT NULL
);

CREATE TABLE "User" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_User" PRIMARY KEY AUTOINCREMENT,
    "Username" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    "PasswordHash" TEXT NOT NULL,
    "Role" INTEGER NOT NULL,
    "Active" INTEGER NOT NULL,
    "FailedAttempts" INTEGER NOT NULL,
    "LockedUntil" TEXT NULL
);

CREATE TABLE "AuditEvent" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_AuditEvent" PRIMARY KEY AUTOINCREMENT,
    "CreatedAt" TEXT NOT NULL,
    "UserId" INTEGER NULL,
    "Action" TEXT NOT NULL,
    "Reference" TEXT NOT NULL,
    "Detail" TEXT NOT NULL,
    CONSTRAINT "FK_AuditEvent_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "CashSession" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_CashSession" PRIMARY KEY AUTOINCREMENT,
    "Folio" TEXT NOT NULL,
    "UserId" INTEGER NOT NULL,
    "OpenedAt" TEXT NOT NULL,
    "ClosedAt" TEXT NULL,
    "OpeningCents" INTEGER NOT NULL,
    "ExpectedCents" INTEGER NULL,
    "CountedCents" INTEGER NULL,
    "Notes" TEXT NOT NULL,
    CONSTRAINT "FK_CashSession_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "Purchase" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Purchase" PRIMARY KEY AUTOINCREMENT,
    "SupplierId" INTEGER NOT NULL,
    "Folio" TEXT NOT NULL,
    "SupplierReference" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "UserId" INTEGER NOT NULL,
    "TotalCents" INTEGER NOT NULL,
    CONSTRAINT "FK_Purchase_Supplier_SupplierId" FOREIGN KEY ("SupplierId") REFERENCES "Supplier" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Purchase_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "StockMovement" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_StockMovement" PRIMARY KEY AUTOINCREMENT,
    "ProductId" INTEGER NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "UserId" INTEGER NOT NULL,
    "Kind" TEXT NOT NULL,
    "Reference" TEXT NOT NULL,
    "Reason" TEXT NOT NULL,
    "DeltaUnits" INTEGER NOT NULL,
    "BalanceUnits" INTEGER NOT NULL,
    CONSTRAINT "FK_StockMovement_Product_ProductId" FOREIGN KEY ("ProductId") REFERENCES "Product" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_StockMovement_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "CashMovement" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_CashMovement" PRIMARY KEY AUTOINCREMENT,
    "CashSessionId" INTEGER NOT NULL,
    "UserId" INTEGER NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "AmountCents" INTEGER NOT NULL,
    "Kind" TEXT NOT NULL,
    "Reference" TEXT NOT NULL,
    "Reason" TEXT NOT NULL,
    CONSTRAINT "FK_CashMovement_CashSession_CashSessionId" FOREIGN KEY ("CashSessionId") REFERENCES "CashSession" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_CashMovement_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "Sale" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Sale" PRIMARY KEY AUTOINCREMENT,
    "Folio" TEXT NOT NULL,
    "RequestId" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "UserId" INTEGER NOT NULL,
    "Cashier" TEXT NOT NULL,
    "CustomerId" INTEGER NULL,
    "CustomerName" TEXT NOT NULL,
    "CashSessionId" INTEGER NOT NULL,
    "TotalCents" INTEGER NOT NULL,
    "TaxCents" INTEGER NOT NULL,
    "DiscountCents" INTEGER NOT NULL,
    "ChangeCents" INTEGER NOT NULL,
    "TenderedCents" INTEGER NOT NULL,
    "Cancelled" INTEGER NOT NULL,
    CONSTRAINT "CK_Sale_Total" CHECK (TotalCents >= 0 AND TaxCents >= 0 AND ChangeCents >= 0),
    CONSTRAINT "FK_Sale_CashSession_CashSessionId" FOREIGN KEY ("CashSessionId") REFERENCES "CashSession" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Sale_Customer_CustomerId" FOREIGN KEY ("CustomerId") REFERENCES "Customer" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Sale_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "PurchaseLine" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_PurchaseLine" PRIMARY KEY AUTOINCREMENT,
    "PurchaseId" INTEGER NOT NULL,
    "ProductId" INTEGER NOT NULL,
    "QuantityUnits" INTEGER NOT NULL,
    "CostCents" INTEGER NOT NULL,
    CONSTRAINT "CK_PurchaseLine" CHECK (QuantityUnits > 0 AND CostCents >= 0),
    CONSTRAINT "FK_PurchaseLine_Product_ProductId" FOREIGN KEY ("ProductId") REFERENCES "Product" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_PurchaseLine_Purchase_PurchaseId" FOREIGN KEY ("PurchaseId") REFERENCES "Purchase" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "CreditEntry" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_CreditEntry" PRIMARY KEY AUTOINCREMENT,
    "CustomerId" INTEGER NOT NULL,
    "SaleId" INTEGER NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "DueDate" TEXT NULL,
    "AmountCents" INTEGER NOT NULL,
    "Method" INTEGER NOT NULL,
    "Kind" TEXT NOT NULL,
    "Reference" TEXT NOT NULL,
    "UserId" INTEGER NOT NULL,
    CONSTRAINT "FK_CreditEntry_Customer_CustomerId" FOREIGN KEY ("CustomerId") REFERENCES "Customer" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_CreditEntry_Sale_SaleId" FOREIGN KEY ("SaleId") REFERENCES "Sale" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_CreditEntry_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "PaymentRecord" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_PaymentRecord" PRIMARY KEY AUTOINCREMENT,
    "SaleId" INTEGER NOT NULL,
    "Method" INTEGER NOT NULL,
    "AmountCents" INTEGER NOT NULL,
    "Reference" TEXT NOT NULL,
    CONSTRAINT "CK_Payment_Amount" CHECK (AmountCents >= 0),
    CONSTRAINT "FK_PaymentRecord_Sale_SaleId" FOREIGN KEY ("SaleId") REFERENCES "Sale" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "Quote" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Quote" PRIMARY KEY AUTOINCREMENT,
    "Folio" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "ValidUntil" TEXT NOT NULL,
    "CustomerId" INTEGER NULL,
    "CustomerName" TEXT NOT NULL,
    "Notes" TEXT NOT NULL,
    "TotalCents" INTEGER NOT NULL,
    "TaxCents" INTEGER NOT NULL,
    "SaleId" INTEGER NULL,
    "UserId" INTEGER NOT NULL,
    CONSTRAINT "FK_Quote_Customer_CustomerId" FOREIGN KEY ("CustomerId") REFERENCES "Customer" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Quote_Sale_SaleId" FOREIGN KEY ("SaleId") REFERENCES "Sale" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Quote_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "SaleLine" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_SaleLine" PRIMARY KEY AUTOINCREMENT,
    "SaleId" INTEGER NOT NULL,
    "ProductId" INTEGER NOT NULL,
    "Sku" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    "Category" TEXT NOT NULL,
    "Unit" INTEGER NOT NULL,
    "QuantityUnits" INTEGER NOT NULL,
    "PriceCents" INTEGER NOT NULL,
    "CostCents" INTEGER NULL,
    "TaxBasisPoints" INTEGER NOT NULL,
    "TotalCents" INTEGER NOT NULL,
    "TaxCents" INTEGER NOT NULL,
    "ReturnedUnits" INTEGER NOT NULL,
    "RefundedCents" INTEGER NOT NULL,
    CONSTRAINT "CK_SaleLine_Quantity" CHECK (QuantityUnits > 0 AND ReturnedUnits >= 0 AND ReturnedUnits <= QuantityUnits AND RefundedCents >= 0 AND RefundedCents <= TotalCents),
    CONSTRAINT "FK_SaleLine_Product_ProductId" FOREIGN KEY ("ProductId") REFERENCES "Product" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_SaleLine_Sale_SaleId" FOREIGN KEY ("SaleId") REFERENCES "Sale" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "SaleReturn" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_SaleReturn" PRIMARY KEY AUTOINCREMENT,
    "SaleId" INTEGER NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "UserId" INTEGER NOT NULL,
    "Folio" TEXT NOT NULL,
    "Reason" TEXT NOT NULL,
    "TotalCents" INTEGER NOT NULL,
    "CreditReductionCents" INTEGER NOT NULL,
    "RefundMethod" INTEGER NOT NULL,
    CONSTRAINT "FK_SaleReturn_Sale_SaleId" FOREIGN KEY ("SaleId") REFERENCES "Sale" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_SaleReturn_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "QuoteLine" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_QuoteLine" PRIMARY KEY AUTOINCREMENT,
    "QuoteId" INTEGER NOT NULL,
    "ProductId" INTEGER NOT NULL,
    "Name" TEXT NOT NULL,
    "Unit" INTEGER NOT NULL,
    "QuantityUnits" INTEGER NOT NULL,
    "PriceCents" INTEGER NOT NULL,
    "TaxBasisPoints" INTEGER NOT NULL,
    "TotalCents" INTEGER NOT NULL,
    CONSTRAINT "FK_QuoteLine_Product_ProductId" FOREIGN KEY ("ProductId") REFERENCES "Product" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_QuoteLine_Quote_QuoteId" FOREIGN KEY ("QuoteId") REFERENCES "Quote" ("Id") ON DELETE CASCADE
);

CREATE TABLE "ReturnLine" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_ReturnLine" PRIMARY KEY AUTOINCREMENT,
    "SaleReturnId" INTEGER NOT NULL,
    "SaleLineId" INTEGER NOT NULL,
    "QuantityUnits" INTEGER NOT NULL,
    "AmountCents" INTEGER NOT NULL,
    CONSTRAINT "FK_ReturnLine_SaleLine_SaleLineId" FOREIGN KEY ("SaleLineId") REFERENCES "SaleLine" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_ReturnLine_SaleReturn_SaleReturnId" FOREIGN KEY ("SaleReturnId") REFERENCES "SaleReturn" ("Id") ON DELETE RESTRICT
);

CREATE VIRTUAL TABLE ProductSearch USING fts5(Sku, Barcode, Name, Category, Brand, Size, Aliases, content='Product', content_rowid='Id', tokenize='unicode61 remove_diacritics 2');

CREATE TABLE "HeldSale" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_HeldSale" PRIMARY KEY AUTOINCREMENT,
    "Folio" TEXT NOT NULL,
    "Label" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "UserId" INTEGER NOT NULL,
    "CustomerId" INTEGER NULL,
    "SaleId" INTEGER NULL,
    "DiscountParts" INTEGER NOT NULL,
    "Discarded" INTEGER NOT NULL,
    CONSTRAINT "FK_HeldSale_Customer_CustomerId" FOREIGN KEY ("CustomerId") REFERENCES "Customer" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_HeldSale_Sale_SaleId" FOREIGN KEY ("SaleId") REFERENCES "Sale" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_HeldSale_User_UserId" FOREIGN KEY ("UserId") REFERENCES "User" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "HeldSaleLine" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_HeldSaleLine" PRIMARY KEY AUTOINCREMENT,
    "HeldSaleId" INTEGER NOT NULL,
    "ProductId" INTEGER NOT NULL,
    "QuantityUnits" INTEGER NOT NULL,
    "Wholesale" INTEGER NOT NULL,
    "OriginalPriceCents" INTEGER NOT NULL,
    CONSTRAINT "CK_Held_Quantity" CHECK (QuantityUnits > 0 AND OriginalPriceCents >= 0),
    CONSTRAINT "FK_HeldSaleLine_HeldSale_HeldSaleId" FOREIGN KEY ("HeldSaleId") REFERENCES "HeldSale" ("Id") ON DELETE CASCADE,
    CONSTRAINT "FK_HeldSaleLine_Product_ProductId" FOREIGN KEY ("ProductId") REFERENCES "Product" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "BackupRecord" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_BackupRecord" PRIMARY KEY AUTOINCREMENT,
    "CreatedAt" TEXT NOT NULL,
    "FilePath" TEXT NOT NULL,
    "Kind" TEXT NOT NULL,
    "Successful" INTEGER NOT NULL,
    "FileBytes" INTEGER NOT NULL,
    "Sha256" TEXT NOT NULL,
    "Result" TEXT NOT NULL
);

CREATE INDEX "IX_AuditEvent_CreatedAt" ON "AuditEvent" ("CreatedAt");

CREATE INDEX "IX_AuditEvent_UserId" ON "AuditEvent" ("UserId");

CREATE INDEX "IX_CashMovement_CashSessionId_CreatedAt" ON "CashMovement" ("CashSessionId", "CreatedAt");

CREATE INDEX "IX_CashMovement_UserId" ON "CashMovement" ("UserId");

CREATE UNIQUE INDEX "IX_CashSession_Folio" ON "CashSession" ("Folio");

CREATE INDEX "IX_CashSession_UserId" ON "CashSession" ("UserId");

CREATE INDEX "IX_CreditEntry_CustomerId_SaleId" ON "CreditEntry" ("CustomerId", "SaleId");

CREATE INDEX "IX_CreditEntry_SaleId" ON "CreditEntry" ("SaleId");

CREATE INDEX "IX_CreditEntry_UserId" ON "CreditEntry" ("UserId");

CREATE INDEX "IX_PaymentRecord_SaleId" ON "PaymentRecord" ("SaleId");

CREATE INDEX "IX_Product_Active_Name" ON "Product" ("Active", "Name");

CREATE UNIQUE INDEX "IX_Product_Barcode" ON "Product" ("Barcode");

CREATE INDEX "IX_Product_Category" ON "Product" ("Category");

CREATE UNIQUE INDEX "IX_Product_Sku" ON "Product" ("Sku");

CREATE UNIQUE INDEX "IX_Purchase_SupplierId_SupplierReference" ON "Purchase" ("SupplierId", "SupplierReference");

CREATE INDEX "IX_Purchase_UserId" ON "Purchase" ("UserId");

CREATE INDEX "IX_PurchaseLine_ProductId" ON "PurchaseLine" ("ProductId");

CREATE INDEX "IX_PurchaseLine_PurchaseId" ON "PurchaseLine" ("PurchaseId");

CREATE INDEX "IX_Quote_CustomerId" ON "Quote" ("CustomerId");

CREATE UNIQUE INDEX "IX_Quote_Folio" ON "Quote" ("Folio");

CREATE INDEX "IX_Quote_SaleId" ON "Quote" ("SaleId");

CREATE INDEX "IX_Quote_UserId" ON "Quote" ("UserId");

CREATE INDEX "IX_QuoteLine_ProductId" ON "QuoteLine" ("ProductId");

CREATE INDEX "IX_QuoteLine_QuoteId" ON "QuoteLine" ("QuoteId");

CREATE INDEX "IX_ReturnLine_SaleLineId" ON "ReturnLine" ("SaleLineId");

CREATE INDEX "IX_ReturnLine_SaleReturnId" ON "ReturnLine" ("SaleReturnId");

CREATE INDEX "IX_Sale_CashSessionId" ON "Sale" ("CashSessionId");

CREATE INDEX "IX_Sale_CreatedAt" ON "Sale" ("CreatedAt");

CREATE INDEX "IX_Sale_CustomerId" ON "Sale" ("CustomerId");

CREATE UNIQUE INDEX "IX_Sale_Folio" ON "Sale" ("Folio");

CREATE UNIQUE INDEX "IX_Sale_RequestId" ON "Sale" ("RequestId");

CREATE INDEX "IX_Sale_UserId" ON "Sale" ("UserId");

CREATE INDEX "IX_SaleLine_ProductId" ON "SaleLine" ("ProductId");

CREATE INDEX "IX_SaleLine_SaleId" ON "SaleLine" ("SaleId");

CREATE INDEX "IX_SaleReturn_CreatedAt" ON "SaleReturn" ("CreatedAt");

CREATE INDEX "IX_SaleReturn_SaleId" ON "SaleReturn" ("SaleId");

CREATE INDEX "IX_SaleReturn_UserId" ON "SaleReturn" ("UserId");

CREATE UNIQUE INDEX "IX_Sequence_Key" ON "Sequence" ("Key");

CREATE INDEX "IX_StockMovement_ProductId_CreatedAt" ON "StockMovement" ("ProductId", "CreatedAt");

CREATE INDEX "IX_StockMovement_UserId" ON "StockMovement" ("UserId");

CREATE UNIQUE INDEX "IX_User_Username" ON "User" ("Username");

CREATE INDEX "IX_HeldSale_CustomerId" ON "HeldSale" ("CustomerId");

CREATE UNIQUE INDEX "IX_HeldSale_Folio" ON "HeldSale" ("Folio");

CREATE INDEX "IX_HeldSale_SaleId" ON "HeldSale" ("SaleId");

CREATE INDEX "IX_HeldSale_UserId" ON "HeldSale" ("UserId");

CREATE INDEX "IX_HeldSaleLine_HeldSaleId" ON "HeldSaleLine" ("HeldSaleId");

CREATE INDEX "IX_HeldSaleLine_ProductId" ON "HeldSaleLine" ("ProductId");

CREATE UNIQUE INDEX IX_CashSession_OneOpen ON CashSession((1)) WHERE ClosedAt IS NULL;

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

INSERT INTO __EFMigrationsHistory VALUES ('20261030212631_InitialCommerce','10.0.14');

INSERT INTO __EFMigrationsHistory VALUES ('20261030215022_ProductSearchFields','10.0.14');

INSERT INTO __EFMigrationsHistory VALUES ('20261030215143_HeldSales','10.0.14');

INSERT INTO __EFMigrationsHistory VALUES ('20261030215746_BackupHistoryAndEmbeddedLogo','10.0.14');
