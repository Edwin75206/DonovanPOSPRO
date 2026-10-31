use crate::error::{AppError, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rusqlite::{Connection, OpenFlags, Params, Row, types::ValueRef};
use serde_json::{Map, Value, json};
use std::{path::Path, time::Duration};

pub const SCHEMA_VERSION: i64 = 5;
pub const LEGACY_SCHEMA: &str = include_str!("../migrations/001_legacy.sql");
const DESKTOP_SCHEMA: &str = include_str!("../migrations/002_desktop.sql");
const APPEARANCE_SCHEMA: &str = include_str!("../migrations/003_appearance.sql");
const DOCUMENTS_SCHEMA: &str = include_str!("../migrations/004_business_documents.sql");
const TICKET_SCHEMA: &str = include_str!("../migrations/005_ticket_profile.sql");
pub const LEGACY_LAST: &str = "20261030215746_BackupHistoryAndEmbeddedLogo";

pub fn connect(path: &Path, readonly: bool) -> Result<Connection> {
    let flags = if readonly {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    };
    let db = Connection::open_with_flags(path, flags | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    db.set_limit(
        rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
        16 * 1024 * 1024,
    )?;
    db.set_limit(
        rusqlite::limits::Limit::SQLITE_LIMIT_SQL_LENGTH,
        1024 * 1024,
    )?;
    db.busy_timeout(Duration::from_secs(5))?;
    db.execute_batch("PRAGMA foreign_keys=ON;")?;
    if !readonly {
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
    }
    Ok(db)
}
pub fn schema(db: &Connection) -> Result<i64> {
    Ok(db.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}
pub fn table_exists(db: &Connection, name: &str) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}
pub fn compatible(db: &Connection) -> Result<i64> {
    let snapshot = if db.is_autocommit() {
        Some(db.unchecked_transaction()?)
    } else {
        None
    };
    let version = schema(db)?;
    ensure(
        version == 0 || (2..=SCHEMA_VERSION).contains(&version),
        "INCOMPATIBLE_DATABASE",
        "Esta base pertenece a otra versión. No se ha modificado. Instala una versión compatible de Donovan POS Pro.",
    )?;
    ensure(
        table_exists(db, "__EFMigrationsHistory")?,
        "INCOMPATIBLE_DATABASE",
        "El archivo no es una base compatible de Donovan POS Pro.",
    )?;
    let migrations: Vec<String> = db
        .prepare("SELECT MigrationId FROM __EFMigrationsHistory ORDER BY MigrationId")?
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    ensure(
        migrations
            == [
                "20261030212631_InitialCommerce",
                "20261030215022_ProductSearchFields",
                "20261030215143_HeldSales",
                LEGACY_LAST,
            ],
        "INCOMPATIBLE_DATABASE",
        "La versión de esta base no está reconocida. Conserva el original y solicita una migración compatible.",
    )?;
    // Compare every legacy business column and foreign key with the preserved reference schema.
    let reference = Connection::open_in_memory()?;
    reference.execute_batch(LEGACY_SCHEMA)?;
    if version >= 2 {
        reference.execute_batch(DESKTOP_SCHEMA)?;
    }
    if version >= 3 {
        reference.execute_batch(APPEARANCE_SCHEMA)?;
    }
    if version >= 4 {
        reference.execute_batch(DOCUMENTS_SCHEMA)?;
    }
    if version >= 5 {
        reference.execute_batch(TICKET_SCHEMA)?;
    }
    for name in reference.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'ProductSearch%' AND name NOT LIKE '__EF%'")?.query_map([], |r| r.get::<_,String>(0))? {
        let name = name?;
        let expected = all(&reference, &format!("PRAGMA table_info(\"{name}\")"), [])?;
        let actual = all(db, &format!("PRAGMA table_info(\"{name}\")"), [])?;
        for column in expected {
            ensure(actual.iter().any(|a| a["name"] == column["name"] && a["type"] == column["type"] && a["pk"] == column["pk"]), "INCOMPATIBLE_DATABASE", "La estructura de la base está incompleta o fue modificada. No se ha conectado.")?;
        }
        ensure(all(db,&format!("PRAGMA foreign_key_list(\"{name}\")"),[])? == all(&reference,&format!("PRAGMA foreign_key_list(\"{name}\")"),[])?, "INCOMPATIBLE_DATABASE", "Las relaciones de la base no corresponden a esta versión.")?;
    }
    // FTS5 maintenance requires trusted triggers. Accept only the exact known schema objects.
    let schema_objects = |connection: &Connection| -> Result<Vec<(String, String)>> {
        let rows:Vec<(String,String)>=connection.prepare("SELECT name,sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name")?.query_map([],|r|Ok((r.get(0)?,r.get::<_,String>(1)?.split_whitespace().collect::<Vec<_>>().join(" "))))?.collect::<std::result::Result<_,_>>()?;
        Ok(rows)
    };
    ensure(
        schema_objects(db)? == schema_objects(&reference)?,
        "INCOMPATIBLE_DATABASE",
        "Los índices, tablas o reglas de esta base fueron modificados. No se ha conectado ni alterado el archivo original.",
    )?;
    if version >= 2 {
        ensure(
            table_exists(db, "AppMeta")?
                && table_exists(db, "UserSession")?
                && table_exists(db, "UserPreferences")?,
            "INCOMPATIBLE_DATABASE",
            "Faltan tablas necesarias para esta versión.",
        )?;
        ensure(
            db.query_row("SELECT Value FROM AppMeta WHERE Key='product'", [], |r| {
                r.get::<_, String>(0)
            })? == "donovan-pos",
            "INCOMPATIBLE_DATABASE",
            "La base pertenece a otra aplicación.",
        )?;
        let identity = all(db, "SELECT Value FROM AppMeta WHERE Key='databaseId'", [])?;
        ensure(
            identity
                .first()
                .and_then(|v| v["value"].as_str())
                .is_some_and(|v| uuid::Uuid::parse_str(v).is_ok()),
            "INCONSISTENT_DATA",
            "La identificación de esta base está incompleta. Selecciona una copia válida.",
        )?;
        let applied: Vec<i64> = db
            .prepare("SELECT Version FROM NativeMigration ORDER BY Version")?
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ensure(
            applied == (2..=version).collect::<Vec<_>>(),
            "INCOMPATIBLE_DATABASE",
            "El historial de migraciones de esta base está incompleto.",
        )?;
    }
    integrity(db)?;
    if let Some(snapshot) = snapshot {
        snapshot.commit()?;
    }
    Ok(version)
}
pub fn migrate(db: &mut Connection, now: &str) -> Result<()> {
    let fresh = db.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        [],
        |r| r.get::<_, i64>(0),
    )? == 0;
    if !fresh {
        compatible(db)?;
    }
    let current = if fresh { 0 } else { schema(db)? };
    if current == SCHEMA_VERSION {
        return Ok(());
    }
    let tx = db.transaction()?;
    if fresh {
        tx.execute_batch(LEGACY_SCHEMA)?;
    }
    if current < 2 {
        tx.execute_batch(DESKTOP_SCHEMA)?;
        tx.execute(
            "INSERT INTO AppMeta VALUES ('databaseId',?1)",
            [uuid::Uuid::new_v4().to_string()],
        )?;
        tx.execute("INSERT INTO NativeMigration VALUES (2,?1,'Esquema compatible 1.0, sesiones seguras y preferencias por usuario')", [now])?;
        if !fresh {
            tx.execute_batch("UPDATE UserPreferences SET Theme=COALESCE((SELECT CASE WHEN DarkMode=1 THEN 'dark' ELSE 'light' END FROM StoreSettings LIMIT 1),'system'),ReducedMotion=COALESCE((SELECT ReducedMotion FROM StoreSettings LIMIT 1),0);")?;
        }
    }
    if current < 3 {
        tx.execute_batch(APPEARANCE_SCHEMA)?;
        tx.execute("INSERT INTO NativeMigration VALUES (3,?1,'Apariencia personal versionada y fondo local por usuario')",[now])?;
    }
    if current < 4 {
        tx.execute_batch(DOCUMENTS_SCHEMA)?;
        tx.execute("INSERT INTO NativeMigration VALUES (4,?1,'Presentación del negocio y detalle exacto de compras por empaque')", [now])?;
    }
    if current < 5 {
        tx.execute_batch(TICKET_SCHEMA)?;
        tx.execute("INSERT INTO NativeMigration VALUES (5,?1,'Perfil de ticket, márgenes y copias de impresión')", [now])?;
    }
    tx.commit()?;
    integrity(db)
}
pub fn integrity(db: &Connection) -> Result<()> {
    let snapshot = if db.is_autocommit() {
        Some(db.unchecked_transaction()?)
    } else {
        None
    };
    let results: Vec<String> = db
        .prepare("PRAGMA integrity_check")?
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    ensure(
        results == ["ok"],
        "CORRUPT_DATABASE",
        "La comprobación encontró daños en la base. Conserva este archivo y restaura un respaldo verificado.",
    )?;
    ensure(
        all(db, "PRAGMA foreign_key_check", [])?.is_empty(),
        "CORRUPT_DATABASE",
        "Hay registros con relaciones inválidas. La base no puede utilizarse hasta recuperar una copia válida.",
    )?;
    business_integrity(db)?;
    if let Some(snapshot) = snapshot {
        snapshot.commit()?;
    }
    Ok(())
}
fn business_integrity(db: &Connection) -> Result<()> {
    if schema(db)? >= 4 {
        ensure(
            scalar(
                db,
                "SELECT COUNT(*) FROM Purchase p JOIN PurchaseDetails d ON d.PurchaseId=p.Id WHERE p.TotalCents<>COALESCE((SELECT SUM(x.TotalCents) FROM PurchaseLine l JOIN PurchaseLineDetails x ON x.PurchaseLineId=l.Id WHERE l.PurchaseId=p.Id),0) OR (SELECT COUNT(*) FROM PurchaseLine WHERE PurchaseId=p.Id)<>(SELECT COUNT(*) FROM PurchaseLine l JOIN PurchaseLineDetails x ON x.PurchaseLineId=l.Id WHERE l.PurchaseId=p.Id)",
                [],
            )? == 0,
            "INCONSISTENT_DATA",
            "Una compra no coincide con sus importes originales. Conserva la base y solicita una revisión técnica.",
        )?;
    }
    // These checks also cover legacy records, without rewriting business amounts or history.
    let checks = [
        (
            "SELECT COUNT(*) FROM User WHERE octet_length(Username) NOT BETWEEN 1 AND 200 OR octet_length(Name) NOT BETWEEN 1 AND 400 OR octet_length(PasswordHash)>1024",
            "Hay cuentas con información fuera de los límites admitidos.",
        ),
        (
            "SELECT COUNT(*) FROM User WHERE typeof(Role)<>'integer' OR Role NOT BETWEEN 0 AND 2 OR Active NOT IN (0,1)",
            "Hay cuentas con roles o estados inválidos.",
        ),
        (
            "SELECT CASE WHEN (SELECT COUNT(*) FROM User)>0 AND ((SELECT COUNT(*) FROM User WHERE Active=1 AND Role=0)=0 OR (SELECT COUNT(*) FROM StoreSettings)<>1) THEN 1 ELSE 0 END",
            "Falta un administrador activo o la configuración del negocio.",
        ),
        (
            "SELECT COUNT(*) FROM Product WHERE typeof(StockUnits)<>'integer' OR typeof(PriceCents)<>'integer' OR typeof(WholesaleCents)<>'integer' OR (CostCents IS NOT NULL AND typeof(CostCents)<>'integer') OR Unit NOT BETWEEN 0 AND 5 OR StockUnits<0 OR MinimumUnits<0 OR PackUnits<=0 OR (Unit IN (0,4,5) AND (StockUnits%10000<>0 OR MinimumUnits%10000<>0 OR PackUnits%10000<>0))",
            "Hay productos con cantidades o unidades inconsistentes.",
        ),
        (
            "SELECT COUNT(*) FROM Sale s WHERE TotalCents<>COALESCE((SELECT SUM(TotalCents) FROM SaleLine WHERE SaleId=s.Id),0) OR TaxCents<>COALESCE((SELECT SUM(TaxCents) FROM SaleLine WHERE SaleId=s.Id),0) OR TotalCents<>COALESCE((SELECT SUM(AmountCents) FROM PaymentRecord WHERE SaleId=s.Id),0) OR NOT EXISTS(SELECT 1 FROM SaleLine WHERE SaleId=s.Id)",
            "Los importes de una venta no coinciden con sus partidas o pagos.",
        ),
        (
            "SELECT COUNT(*) FROM SaleLine l WHERE ReturnedUnits<>COALESCE((SELECT SUM(QuantityUnits) FROM ReturnLine WHERE SaleLineId=l.Id),0) OR RefundedCents<>COALESCE((SELECT SUM(AmountCents) FROM ReturnLine WHERE SaleLineId=l.Id),0)",
            "El historial de devoluciones presenta diferencias de cantidades o importes.",
        ),
        (
            "SELECT COUNT(*) FROM SaleReturn r WHERE TotalCents<>COALESCE((SELECT SUM(AmountCents) FROM ReturnLine WHERE SaleReturnId=r.Id),0) OR CreditReductionCents<0 OR CreditReductionCents>TotalCents OR RefundMethod NOT BETWEEN 0 AND 2",
            "Una devolución tiene importes inconsistentes.",
        ),
        (
            "SELECT COUNT(*) FROM ReturnLine r JOIN SaleLine l ON l.Id=r.SaleLineId JOIN SaleReturn h ON h.Id=r.SaleReturnId WHERE l.SaleId<>h.SaleId",
            "Una devolución no corresponde a la venta indicada.",
        ),
        (
            "SELECT COUNT(*) FROM (SELECT SaleId FROM CreditEntry GROUP BY SaleId HAVING SUM(AmountCents)<0)",
            "Una cuenta por cobrar presenta un saldo negativo.",
        ),
        (
            "SELECT COUNT(*) FROM CreditEntry c JOIN Sale s ON s.Id=c.SaleId WHERE s.CustomerId IS NULL OR c.CustomerId<>s.CustomerId",
            "Un movimiento de crédito no corresponde al cliente de la venta.",
        ),
        (
            "SELECT COUNT(*) FROM Quote q WHERE TotalCents<>COALESCE((SELECT SUM(TotalCents) FROM QuoteLine WHERE QuoteId=q.Id),0) OR NOT EXISTS(SELECT 1 FROM QuoteLine WHERE QuoteId=q.Id)",
            "Una cotización no coincide con sus partidas.",
        ),
    ];
    for (query, message) in checks {
        ensure(
            scalar(db, query, [])? == 0,
            "INCONSISTENT_DATA",
            &format!(
                "{message} Conserva el archivo y solicita una revisión técnica antes de utilizarlo."
            ),
        )?;
    }
    Ok(())
}
fn row_json(row: &Row<'_>) -> rusqlite::Result<Value> {
    let mut result = Map::new();
    for i in 0..row.as_ref().column_count() {
        let key = row.as_ref().column_name(i)?;
        let mut chars = key.chars();
        let key = chars.next().unwrap_or('_').to_lowercase().to_string() + chars.as_str();
        let value = match row.get_ref(i)? {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(n)
                if [
                    "active",
                    "cancelled",
                    "automaticBackups",
                    "darkMode",
                    "reducedMotion",
                    "successful",
                    "discarded",
                    "wholesale",
                    "locked",
                    "paidFromCash",
                    "sourceExact",
                    "byPack",
                    "autoPrint",
                ]
                .contains(&key.as_str()) =>
            {
                json!(n != 0)
            }
            ValueRef::Integer(n) => json!(n),
            ValueRef::Real(n) => json!(n),
            ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
            ValueRef::Blob(b) => json!(STANDARD.encode(b)),
        };
        result.insert(key, value);
    }
    Ok(Value::Object(result))
}
pub fn all(db: &Connection, sql: &str, params: impl Params) -> Result<Vec<Value>> {
    Ok(db
        .prepare(sql)?
        .query_map(params, row_json)?
        .collect::<std::result::Result<_, _>>()?)
}
pub fn one(db: &Connection, sql: &str, params: impl Params) -> Result<Value> {
    all(db, sql, params)?.into_iter().next().ok_or_else(|| {
        AppError::rule(
            "NOT_FOUND",
            "No encontramos el registro. Actualiza la lista e intenta nuevamente.",
        )
    })
}
pub fn scalar(db: &Connection, sql: &str, params: impl Params) -> Result<i64> {
    Ok(db.query_row(sql, params, |r| r.get(0))?)
}
pub fn audit(
    db: &Connection,
    now: &str,
    user: Option<i64>,
    action: &str,
    reference: &str,
    detail: &str,
) -> Result<()> {
    db.execute(
        "INSERT INTO AuditEvent(CreatedAt,UserId,Action,Reference,Detail) VALUES (?1,?2,?3,?4,?5)",
        rusqlite::params![now, user, action, reference, detail],
    )?;
    Ok(())
}
pub fn folio(db: &Connection, key: &str, prefix: &str) -> Result<String> {
    db.execute("INSERT INTO Sequence(Key,Value) VALUES (?1,1) ON CONFLICT(Key) DO UPDATE SET Value=Value+1",[key])?;
    Ok(format!(
        "{}-{:06}",
        prefix,
        scalar(db, "SELECT Value FROM Sequence WHERE Key=?1", [key])?
    ))
}
