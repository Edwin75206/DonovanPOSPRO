-- MySQL 8.4 / InnoDB. Business schema equivalent to native SQLite version 5.
-- Never apply to an existing or foreign schema.

CREATE TABLE `AppMeta` (
  `Key` VARCHAR(512) NOT NULL,
  `Value` LONGTEXT NOT NULL,
  PRIMARY KEY (`Key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `BackupRecord` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `CreatedAt` LONGTEXT NOT NULL,
  `FilePath` LONGTEXT NOT NULL,
  `Kind` LONGTEXT NOT NULL,
  `Successful` BIGINT NOT NULL,
  `FileBytes` BIGINT NOT NULL,
  `Sha256` LONGTEXT NOT NULL,
  `Result` LONGTEXT NOT NULL,
  PRIMARY KEY (`Id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `Customer` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Name` LONGTEXT NOT NULL,
  `Phone` LONGTEXT NOT NULL,
  `Email` LONGTEXT NOT NULL,
  `Address` LONGTEXT NOT NULL,
  `CreditLimitCents` BIGINT NOT NULL,
  `Active` BIGINT NOT NULL,
  PRIMARY KEY (`Id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `NativeMigration` (
  `Version` BIGINT NOT NULL,
  `AppliedAt` LONGTEXT NOT NULL,
  `Description` LONGTEXT NOT NULL,
  PRIMARY KEY (`Version`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `Product` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Sku` VARCHAR(512) NOT NULL,
  `Barcode` VARCHAR(512) NULL,
  `Name` VARCHAR(512) NOT NULL,
  `Category` VARCHAR(512) NOT NULL,
  `Brand` LONGTEXT NOT NULL,
  `Size` LONGTEXT NOT NULL,
  `Unit` BIGINT NOT NULL,
  `CostCents` BIGINT NULL,
  `PriceCents` BIGINT NOT NULL,
  `WholesaleCents` BIGINT NOT NULL,
  `StockUnits` BIGINT NOT NULL,
  `MinimumUnits` BIGINT NOT NULL,
  `TaxBasisPoints` BIGINT NOT NULL,
  `PackName` LONGTEXT NOT NULL,
  `PackUnits` BIGINT NOT NULL,
  `Active` BIGINT NOT NULL,
  `Aliases` LONGTEXT NOT NULL DEFAULT (''),
  PRIMARY KEY (`Id`),
  CONSTRAINT `CK_Product_1` CHECK (PriceCents >= 0 AND WholesaleCents >= 0 AND (CostCents IS NULL OR CostCents >= 0) AND TaxBasisPoints BETWEEN 0 AND 10000),
  CONSTRAINT `CK_Product_2` CHECK (StockUnits >= 0 AND MinimumUnits >= 0 AND PackUnits > 0),
  UNIQUE KEY `IX_Product_Sku` (`Sku`),
  KEY `IX_Product_Category` (`Category`),
  UNIQUE KEY `IX_Product_Barcode` (`Barcode`),
  KEY `IX_Product_Active_Name` (`Active`,`Name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `RequestReceipt` (
  `RequestId` VARCHAR(512) NOT NULL,
  `Operation` LONGTEXT NOT NULL,
  `ResultJson` LONGTEXT NOT NULL,
  `CreatedAt` LONGTEXT NOT NULL,
  PRIMARY KEY (`RequestId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `Sequence` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Key` VARCHAR(512) NOT NULL,
  `Value` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  UNIQUE KEY `IX_Sequence_Key` (`Key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `StoreSettings` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `StoreName` LONGTEXT NOT NULL,
  `Address` LONGTEXT NOT NULL,
  `Phone` LONGTEXT NOT NULL,
  `Email` LONGTEXT NOT NULL,
  `TaxId` LONGTEXT NOT NULL,
  `LogoPath` LONGTEXT NOT NULL,
  `TicketFooter` LONGTEXT NOT NULL,
  `FolioPrefix` LONGTEXT NOT NULL,
  `DefaultTaxBasisPoints` BIGINT NOT NULL,
  `TicketWidthMm` BIGINT NOT NULL,
  `PrinterName` LONGTEXT NOT NULL,
  `BackupDirectory` LONGTEXT NOT NULL,
  `DarkMode` BIGINT NOT NULL,
  `ReducedMotion` BIGINT NOT NULL,
  `AutomaticBackups` BIGINT NOT NULL DEFAULT (0),
  `BackupIntervalHours` BIGINT NOT NULL DEFAULT (0),
  `LogoData` LONGBLOB NULL,
  PRIMARY KEY (`Id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `Supplier` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Name` LONGTEXT NOT NULL,
  `Phone` LONGTEXT NOT NULL,
  `Email` LONGTEXT NOT NULL,
  `Address` LONGTEXT NOT NULL,
  `Active` BIGINT NOT NULL,
  PRIMARY KEY (`Id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `User` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Username` VARCHAR(512) NOT NULL,
  `Name` LONGTEXT NOT NULL,
  `PasswordHash` LONGTEXT NOT NULL,
  `Role` BIGINT NOT NULL,
  `Active` BIGINT NOT NULL,
  `FailedAttempts` BIGINT NOT NULL,
  `LockedUntil` LONGTEXT NULL,
  `AuthVersion` BIGINT NOT NULL DEFAULT (0),
  PRIMARY KEY (`Id`),
  UNIQUE KEY `IX_User_Username` (`Username`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `__EFMigrationsHistory` (
  `MigrationId` VARCHAR(512) NOT NULL,
  `ProductVersion` LONGTEXT NOT NULL,
  PRIMARY KEY (`MigrationId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `__EFMigrationsLock` (
  `Id` BIGINT NOT NULL,
  `Timestamp` LONGTEXT NOT NULL,
  PRIMARY KEY (`Id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `AuditEvent` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `CreatedAt` VARCHAR(512) NOT NULL,
  `UserId` BIGINT NULL,
  `Action` LONGTEXT NOT NULL,
  `Reference` LONGTEXT NOT NULL,
  `Detail` LONGTEXT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_AuditEvent_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_AuditEvent_UserId` (`UserId`),
  KEY `IX_AuditEvent_CreatedAt` (`CreatedAt`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `CashSession` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Folio` VARCHAR(512) NOT NULL,
  `UserId` BIGINT NOT NULL,
  `OpenedAt` LONGTEXT NOT NULL,
  `ClosedAt` LONGTEXT NULL,
  `OpeningCents` BIGINT NOT NULL,
  `ExpectedCents` BIGINT NULL,
  `CountedCents` BIGINT NULL,
  `Notes` LONGTEXT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_CashSession_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_CashSession_UserId` (`UserId`),
  UNIQUE KEY `IX_CashSession_Folio` (`Folio`),
  UNIQUE KEY `IX_CashSession_OneOpen` ((CASE WHEN `ClosedAt` IS NULL THEN 1 ELSE NULL END))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `Purchase` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `SupplierId` BIGINT NOT NULL,
  `Folio` LONGTEXT NOT NULL,
  `SupplierReference` VARCHAR(512) NOT NULL,
  `CreatedAt` LONGTEXT NOT NULL,
  `UserId` BIGINT NOT NULL,
  `TotalCents` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_Purchase_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_Purchase_SupplierId` FOREIGN KEY (`SupplierId`) REFERENCES `Supplier` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_Purchase_UserId` (`UserId`),
  UNIQUE KEY `IX_Purchase_SupplierId_SupplierReference` (`SupplierId`,`SupplierReference`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `StockMovement` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `ProductId` BIGINT NOT NULL,
  `CreatedAt` VARCHAR(512) NOT NULL,
  `UserId` BIGINT NOT NULL,
  `Kind` LONGTEXT NOT NULL,
  `Reference` LONGTEXT NOT NULL,
  `Reason` LONGTEXT NOT NULL,
  `DeltaUnits` BIGINT NOT NULL,
  `BalanceUnits` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_StockMovement_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_StockMovement_ProductId` FOREIGN KEY (`ProductId`) REFERENCES `Product` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_StockMovement_UserId` (`UserId`),
  KEY `IX_StockMovement_ProductId_CreatedAt` (`ProductId`,`CreatedAt`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `StorePresentation` (
  `StoreId` BIGINT NOT NULL,
  `Subtitle` LONGTEXT NOT NULL DEFAULT (''),
  `AutoPrint` BIGINT NOT NULL DEFAULT (0),
  PRIMARY KEY (`StoreId`),
  CONSTRAINT `FK_StorePresentation_StoreId` FOREIGN KEY (`StoreId`) REFERENCES `StoreSettings` (`Id`) ON DELETE CASCADE ON UPDATE NO ACTION,
  CONSTRAINT `CK_StorePresentation_1` CHECK (AutoPrint IN (0,1))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `TicketProfile` (
  `StoreId` BIGINT NOT NULL,
  `MarginMm` BIGINT NOT NULL DEFAULT (4),
  `Copies` BIGINT NOT NULL DEFAULT (1),
  `Monochrome` BIGINT NOT NULL DEFAULT (1),
  `ShowLogo` BIGINT NOT NULL DEFAULT (1),
  `Legend` LONGTEXT NOT NULL DEFAULT ('Conserva este comprobante.'),
  PRIMARY KEY (`StoreId`),
  CONSTRAINT `FK_TicketProfile_StoreId` FOREIGN KEY (`StoreId`) REFERENCES `StoreSettings` (`Id`) ON DELETE CASCADE ON UPDATE NO ACTION,
  CONSTRAINT `CK_TicketProfile_1` CHECK (MarginMm BETWEEN 2 AND 8),
  CONSTRAINT `CK_TicketProfile_2` CHECK (Copies BETWEEN 1 AND 3),
  CONSTRAINT `CK_TicketProfile_3` CHECK (Monochrome IN (0,1)),
  CONSTRAINT `CK_TicketProfile_4` CHECK (ShowLogo IN (0,1)),
  CONSTRAINT `CK_TicketProfile_5` CHECK (CHAR_LENGTH(Legend)<=500)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `UserAppearance` (
  `UserId` BIGINT NOT NULL,
  `Version` BIGINT NOT NULL DEFAULT (1),
  `ConfigJson` LONGTEXT NOT NULL,
  `BackgroundData` LONGBLOB NULL,
  PRIMARY KEY (`UserId`),
  CONSTRAINT `FK_UserAppearance_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE CASCADE ON UPDATE NO ACTION,
  CONSTRAINT `CK_UserAppearance_1` CHECK (Version=1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `UserPreferences` (
  `UserId` BIGINT NOT NULL,
  `TutorialStatus` LONGTEXT NOT NULL DEFAULT ('pending'),
  `TutorialStep` BIGINT NOT NULL DEFAULT (0),
  `Theme` LONGTEXT NOT NULL DEFAULT ('system'),
  `ReducedMotion` BIGINT NOT NULL DEFAULT (0),
  PRIMARY KEY (`UserId`),
  CONSTRAINT `FK_UserPreferences_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE CASCADE ON UPDATE NO ACTION,
  CONSTRAINT `CK_UserPreferences_1` CHECK (TutorialStatus IN ('pending','skipped','in_progress','completed')),
  CONSTRAINT `CK_UserPreferences_2` CHECK (Theme IN ('system','light','dark'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `UserSession` (
  `TokenHash` VARCHAR(512) NOT NULL,
  `UserId` BIGINT NOT NULL,
  `AuthVersion` BIGINT NOT NULL,
  `IssuedAt` BIGINT NOT NULL,
  `ExpiresAt` BIGINT NOT NULL,
  `LastSeenAt` BIGINT NOT NULL,
  `Locked` BIGINT NOT NULL DEFAULT (0),
  PRIMARY KEY (`TokenHash`),
  CONSTRAINT `FK_UserSession_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE CASCADE ON UPDATE NO ACTION,
  CONSTRAINT `CK_UserSession_1` CHECK (ExpiresAt = IssuedAt + 86400),
  KEY `IX_Session_User` (`UserId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `CashMovement` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `CashSessionId` BIGINT NOT NULL,
  `UserId` BIGINT NOT NULL,
  `CreatedAt` VARCHAR(512) NOT NULL,
  `AmountCents` BIGINT NOT NULL,
  `Kind` LONGTEXT NOT NULL,
  `Reference` LONGTEXT NOT NULL,
  `Reason` LONGTEXT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_CashMovement_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_CashMovement_CashSessionId` FOREIGN KEY (`CashSessionId`) REFERENCES `CashSession` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_CashMovement_UserId` (`UserId`),
  KEY `IX_CashMovement_CashSessionId_CreatedAt` (`CashSessionId`,`CreatedAt`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `PurchaseDetails` (
  `PurchaseId` BIGINT NOT NULL,
  `PaidFromCash` BIGINT NOT NULL,
  PRIMARY KEY (`PurchaseId`),
  CONSTRAINT `FK_PurchaseDetails_PurchaseId` FOREIGN KEY (`PurchaseId`) REFERENCES `Purchase` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `CK_PurchaseDetails_1` CHECK (PaidFromCash IN (0,1))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `PurchaseLine` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `PurchaseId` BIGINT NOT NULL,
  `ProductId` BIGINT NOT NULL,
  `QuantityUnits` BIGINT NOT NULL,
  `CostCents` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_PurchaseLine_PurchaseId` FOREIGN KEY (`PurchaseId`) REFERENCES `Purchase` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_PurchaseLine_ProductId` FOREIGN KEY (`ProductId`) REFERENCES `Product` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `CK_PurchaseLine_1` CHECK (QuantityUnits > 0 AND CostCents >= 0),
  KEY `IX_PurchaseLine_PurchaseId` (`PurchaseId`),
  KEY `IX_PurchaseLine_ProductId` (`ProductId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `Sale` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Folio` VARCHAR(512) NOT NULL,
  `RequestId` VARCHAR(512) NOT NULL,
  `CreatedAt` VARCHAR(512) NOT NULL,
  `UserId` BIGINT NOT NULL,
  `Cashier` LONGTEXT NOT NULL,
  `CustomerId` BIGINT NULL,
  `CustomerName` LONGTEXT NOT NULL,
  `CashSessionId` BIGINT NOT NULL,
  `TotalCents` BIGINT NOT NULL,
  `TaxCents` BIGINT NOT NULL,
  `DiscountCents` BIGINT NOT NULL,
  `ChangeCents` BIGINT NOT NULL,
  `TenderedCents` BIGINT NOT NULL,
  `Cancelled` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_Sale_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_Sale_CustomerId` FOREIGN KEY (`CustomerId`) REFERENCES `Customer` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_Sale_CashSessionId` FOREIGN KEY (`CashSessionId`) REFERENCES `CashSession` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `CK_Sale_1` CHECK (TotalCents >= 0 AND TaxCents >= 0 AND ChangeCents >= 0),
  KEY `IX_Sale_UserId` (`UserId`),
  UNIQUE KEY `IX_Sale_RequestId` (`RequestId`),
  UNIQUE KEY `IX_Sale_Folio` (`Folio`),
  KEY `IX_Sale_CustomerId` (`CustomerId`),
  KEY `IX_Sale_CreatedAt` (`CreatedAt`),
  KEY `IX_Sale_CashSessionId` (`CashSessionId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `CreditEntry` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `CustomerId` BIGINT NOT NULL,
  `SaleId` BIGINT NOT NULL,
  `CreatedAt` LONGTEXT NOT NULL,
  `DueDate` LONGTEXT NULL,
  `AmountCents` BIGINT NOT NULL,
  `Method` BIGINT NOT NULL,
  `Kind` LONGTEXT NOT NULL,
  `Reference` LONGTEXT NOT NULL,
  `UserId` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_CreditEntry_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_CreditEntry_SaleId` FOREIGN KEY (`SaleId`) REFERENCES `Sale` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_CreditEntry_CustomerId` FOREIGN KEY (`CustomerId`) REFERENCES `Customer` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_CreditEntry_UserId` (`UserId`),
  KEY `IX_CreditEntry_SaleId` (`SaleId`),
  KEY `IX_CreditEntry_CustomerId_SaleId` (`CustomerId`,`SaleId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `HeldSale` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Folio` VARCHAR(512) NOT NULL,
  `Label` LONGTEXT NOT NULL,
  `CreatedAt` LONGTEXT NOT NULL,
  `UserId` BIGINT NOT NULL,
  `CustomerId` BIGINT NULL,
  `SaleId` BIGINT NULL,
  `DiscountParts` BIGINT NOT NULL,
  `Discarded` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_HeldSale_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_HeldSale_SaleId` FOREIGN KEY (`SaleId`) REFERENCES `Sale` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_HeldSale_CustomerId` FOREIGN KEY (`CustomerId`) REFERENCES `Customer` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_HeldSale_UserId` (`UserId`),
  KEY `IX_HeldSale_SaleId` (`SaleId`),
  UNIQUE KEY `IX_HeldSale_Folio` (`Folio`),
  KEY `IX_HeldSale_CustomerId` (`CustomerId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `PaymentRecord` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `SaleId` BIGINT NOT NULL,
  `Method` BIGINT NOT NULL,
  `AmountCents` BIGINT NOT NULL,
  `Reference` LONGTEXT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_PaymentRecord_SaleId` FOREIGN KEY (`SaleId`) REFERENCES `Sale` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `CK_PaymentRecord_1` CHECK (AmountCents >= 0),
  KEY `IX_PaymentRecord_SaleId` (`SaleId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `PurchaseLineDetails` (
  `PurchaseLineId` BIGINT NOT NULL,
  `ProductName` LONGTEXT NOT NULL,
  `ProductSku` LONGTEXT NOT NULL,
  `Unit` BIGINT NOT NULL,
  `EnteredQuantityUnits` BIGINT NOT NULL,
  `EnteredCostCents` BIGINT NOT NULL,
  `ByPack` BIGINT NOT NULL,
  `PackName` LONGTEXT NOT NULL,
  `PackUnits` BIGINT NOT NULL,
  `TotalCents` BIGINT NOT NULL,
  PRIMARY KEY (`PurchaseLineId`),
  CONSTRAINT `FK_PurchaseLineDetails_PurchaseLineId` FOREIGN KEY (`PurchaseLineId`) REFERENCES `PurchaseLine` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `CK_PurchaseLineDetails_1` CHECK (Unit BETWEEN 0 AND 5),
  CONSTRAINT `CK_PurchaseLineDetails_2` CHECK (EnteredQuantityUnits>0),
  CONSTRAINT `CK_PurchaseLineDetails_3` CHECK (EnteredCostCents>=0),
  CONSTRAINT `CK_PurchaseLineDetails_4` CHECK (ByPack IN (0,1)),
  CONSTRAINT `CK_PurchaseLineDetails_5` CHECK (PackUnits>0),
  CONSTRAINT `CK_PurchaseLineDetails_6` CHECK (TotalCents>=0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `Quote` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `Folio` VARCHAR(512) NOT NULL,
  `CreatedAt` LONGTEXT NOT NULL,
  `ValidUntil` LONGTEXT NOT NULL,
  `CustomerId` BIGINT NULL,
  `CustomerName` LONGTEXT NOT NULL,
  `Notes` LONGTEXT NOT NULL,
  `TotalCents` BIGINT NOT NULL,
  `TaxCents` BIGINT NOT NULL,
  `SaleId` BIGINT NULL,
  `UserId` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_Quote_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_Quote_SaleId` FOREIGN KEY (`SaleId`) REFERENCES `Sale` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_Quote_CustomerId` FOREIGN KEY (`CustomerId`) REFERENCES `Customer` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_Quote_UserId` (`UserId`),
  KEY `IX_Quote_SaleId` (`SaleId`),
  UNIQUE KEY `IX_Quote_Folio` (`Folio`),
  KEY `IX_Quote_CustomerId` (`CustomerId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `SaleLine` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `SaleId` BIGINT NOT NULL,
  `ProductId` BIGINT NOT NULL,
  `Sku` LONGTEXT NOT NULL,
  `Name` LONGTEXT NOT NULL,
  `Category` LONGTEXT NOT NULL,
  `Unit` BIGINT NOT NULL,
  `QuantityUnits` BIGINT NOT NULL,
  `PriceCents` BIGINT NOT NULL,
  `CostCents` BIGINT NULL,
  `TaxBasisPoints` BIGINT NOT NULL,
  `TotalCents` BIGINT NOT NULL,
  `TaxCents` BIGINT NOT NULL,
  `ReturnedUnits` BIGINT NOT NULL,
  `RefundedCents` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_SaleLine_SaleId` FOREIGN KEY (`SaleId`) REFERENCES `Sale` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_SaleLine_ProductId` FOREIGN KEY (`ProductId`) REFERENCES `Product` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `CK_SaleLine_1` CHECK (QuantityUnits > 0 AND ReturnedUnits >= 0 AND ReturnedUnits <= QuantityUnits AND RefundedCents >= 0 AND RefundedCents <= TotalCents),
  KEY `IX_SaleLine_SaleId` (`SaleId`),
  KEY `IX_SaleLine_ProductId` (`ProductId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `SaleReturn` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `SaleId` BIGINT NOT NULL,
  `CreatedAt` VARCHAR(512) NOT NULL,
  `UserId` BIGINT NOT NULL,
  `Folio` LONGTEXT NOT NULL,
  `Reason` LONGTEXT NOT NULL,
  `TotalCents` BIGINT NOT NULL,
  `CreditReductionCents` BIGINT NOT NULL,
  `RefundMethod` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_SaleReturn_UserId` FOREIGN KEY (`UserId`) REFERENCES `User` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_SaleReturn_SaleId` FOREIGN KEY (`SaleId`) REFERENCES `Sale` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_SaleReturn_UserId` (`UserId`),
  KEY `IX_SaleReturn_SaleId` (`SaleId`),
  KEY `IX_SaleReturn_CreatedAt` (`CreatedAt`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `HeldSaleLine` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `HeldSaleId` BIGINT NOT NULL,
  `ProductId` BIGINT NOT NULL,
  `QuantityUnits` BIGINT NOT NULL,
  `Wholesale` BIGINT NOT NULL,
  `OriginalPriceCents` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_HeldSaleLine_ProductId` FOREIGN KEY (`ProductId`) REFERENCES `Product` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_HeldSaleLine_HeldSaleId` FOREIGN KEY (`HeldSaleId`) REFERENCES `HeldSale` (`Id`) ON DELETE CASCADE ON UPDATE NO ACTION,
  CONSTRAINT `CK_HeldSaleLine_1` CHECK (QuantityUnits > 0 AND OriginalPriceCents >= 0),
  KEY `IX_HeldSaleLine_ProductId` (`ProductId`),
  KEY `IX_HeldSaleLine_HeldSaleId` (`HeldSaleId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `QuoteLine` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `QuoteId` BIGINT NOT NULL,
  `ProductId` BIGINT NOT NULL,
  `Name` LONGTEXT NOT NULL,
  `Unit` BIGINT NOT NULL,
  `QuantityUnits` BIGINT NOT NULL,
  `PriceCents` BIGINT NOT NULL,
  `TaxBasisPoints` BIGINT NOT NULL,
  `TotalCents` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_QuoteLine_QuoteId` FOREIGN KEY (`QuoteId`) REFERENCES `Quote` (`Id`) ON DELETE CASCADE ON UPDATE NO ACTION,
  CONSTRAINT `FK_QuoteLine_ProductId` FOREIGN KEY (`ProductId`) REFERENCES `Product` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_QuoteLine_QuoteId` (`QuoteId`),
  KEY `IX_QuoteLine_ProductId` (`ProductId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;

CREATE TABLE `ReturnLine` (
  `Id` BIGINT NOT NULL AUTO_INCREMENT,
  `SaleReturnId` BIGINT NOT NULL,
  `SaleLineId` BIGINT NOT NULL,
  `QuantityUnits` BIGINT NOT NULL,
  `AmountCents` BIGINT NOT NULL,
  PRIMARY KEY (`Id`),
  CONSTRAINT `FK_ReturnLine_SaleReturnId` FOREIGN KEY (`SaleReturnId`) REFERENCES `SaleReturn` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  CONSTRAINT `FK_ReturnLine_SaleLineId` FOREIGN KEY (`SaleLineId`) REFERENCES `SaleLine` (`Id`) ON DELETE RESTRICT ON UPDATE NO ACTION,
  KEY `IX_ReturnLine_SaleReturnId` (`SaleReturnId`),
  KEY `IX_ReturnLine_SaleLineId` (`SaleLineId`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;
