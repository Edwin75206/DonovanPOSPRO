CREATE TABLE TicketProfile (
    StoreId INTEGER NOT NULL PRIMARY KEY REFERENCES StoreSettings(Id) ON DELETE CASCADE,
    MarginMm INTEGER NOT NULL DEFAULT 4 CHECK(MarginMm BETWEEN 2 AND 8),
    Copies INTEGER NOT NULL DEFAULT 1 CHECK(Copies BETWEEN 1 AND 3),
    Monochrome INTEGER NOT NULL DEFAULT 1 CHECK(Monochrome IN (0,1)),
    ShowLogo INTEGER NOT NULL DEFAULT 1 CHECK(ShowLogo IN (0,1)),
    Legend TEXT NOT NULL DEFAULT 'Conserva este comprobante.' CHECK(length(Legend)<=500)
);
PRAGMA user_version=5;
