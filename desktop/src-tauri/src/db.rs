use crate::error::{AppError, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rusqlite::{Connection, OpenFlags, Params, Row, types::ValueRef};
use serde_json::{Map, Value, json};
use std::{path::Path, time::Duration};

pub const SCHEMA_VERSION: i64 = 2;
pub const LEGACY_SCHEMA: &str = include_str!("../migrations/001_legacy.sql");
const DESKTOP_SCHEMA: &str = include_str!("../migrations/002_desktop.sql");
pub const LEGACY_LAST: &str = "20261030215746_BackupHistoryAndEmbeddedLogo";

pub fn connect(path: &Path, readonly: bool) -> Result<Connection> {
    let flags = if readonly { OpenFlags::SQLITE_OPEN_READ_ONLY } else { OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE };
    let db = Connection::open_with_flags(path, flags | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    db.busy_timeout(Duration::from_secs(5))?;
    db.execute_batch("PRAGMA foreign_keys=ON;")?;
    if !readonly { db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?; }
    Ok(db)
}
pub fn schema(db: &Connection) -> Result<i64> { Ok(db.query_row("PRAGMA user_version", [], |r| r.get(0))?) }
pub fn table_exists(db: &Connection, name: &str) -> Result<bool> { Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)", [name], |r| r.get(0))?) }
pub fn compatible(db: &Connection) -> Result<i64> {
    let version = schema(db)?;
    ensure(version == 0 || version == SCHEMA_VERSION, "INCOMPATIBLE_DATABASE", "Esta base pertenece a otra versión. No se ha modificado. Instala una versión compatible de Donovan POS Pro.")?;
    ensure(table_exists(db, "__EFMigrationsHistory")?, "INCOMPATIBLE_DATABASE", "El archivo no es una base compatible de Donovan POS Pro.")?;
    let migrations: Vec<String> = db.prepare("SELECT MigrationId FROM __EFMigrationsHistory ORDER BY MigrationId")?.query_map([], |r| r.get(0))?.collect::<std::result::Result<_,_>>()?;
    ensure(migrations == ["20261030212631_InitialCommerce", "20261030215022_ProductSearchFields", "20261030215143_HeldSales", LEGACY_LAST], "INCOMPATIBLE_DATABASE", "La versión de esta base no está reconocida. Conserva el original y solicita una migración compatible.")?;
    // Compare every legacy business column and foreign key with the preserved reference schema.
    let reference = Connection::open_in_memory()?;
    reference.execute_batch(LEGACY_SCHEMA)?;
    if version==2{reference.execute_batch(DESKTOP_SCHEMA)?;}
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
    let schema_objects=|connection:&Connection|->Result<Vec<(String,String)>>{
        let rows:Vec<(String,String)>=connection.prepare("SELECT name,sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name")?.query_map([],|r|Ok((r.get(0)?,r.get::<_,String>(1)?.split_whitespace().collect::<Vec<_>>().join(" "))))?.collect::<std::result::Result<_,_>>()?;Ok(rows)
    };
    ensure(schema_objects(db)?==schema_objects(&reference)?,"INCOMPATIBLE_DATABASE","Los índices, tablas o reglas de esta base fueron modificados. No se ha conectado ni alterado el archivo original.")?;
    if version == 2 {
        ensure(table_exists(db, "AppMeta")? && table_exists(db, "UserSession")? && table_exists(db,"UserPreferences")?, "INCOMPATIBLE_DATABASE", "Faltan tablas necesarias para esta versión.")?;
        ensure(db.query_row("SELECT Value FROM AppMeta WHERE Key='product'", [], |r| r.get::<_,String>(0))? == "donovan-pos", "INCOMPATIBLE_DATABASE", "La base pertenece a otra aplicación.")?;
    }
    integrity(db)?;
    Ok(version)
}
pub fn migrate(db: &mut Connection, now: &str) -> Result<()> {
    let fresh = db.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |r| r.get::<_,i64>(0))? == 0;
    if !fresh { compatible(db)?; }
    if !fresh && schema(db)? == 2 { return Ok(()); }
    let tx = db.transaction()?;
    if fresh { tx.execute_batch(LEGACY_SCHEMA)?; }
    tx.execute_batch(DESKTOP_SCHEMA)?;
    tx.execute("INSERT INTO AppMeta VALUES ('databaseId',?1)", [uuid::Uuid::new_v4().to_string()])?;
    tx.execute("INSERT INTO NativeMigration VALUES (2,?1,'Esquema compatible 1.0, sesiones seguras y preferencias por usuario')", [now])?;
    tx.commit()?;
    integrity(db)
}
pub fn integrity(db: &Connection) -> Result<()> {
    let results: Vec<String> = db.prepare("PRAGMA integrity_check")?.query_map([], |r| r.get(0))?.collect::<std::result::Result<_,_>>()?;
    ensure(results == ["ok"], "CORRUPT_DATABASE", "La comprobación encontró daños en la base. Conserva este archivo y restaura un respaldo verificado.")?;
    ensure(all(db,"PRAGMA foreign_key_check",[])?.is_empty(), "CORRUPT_DATABASE", "Hay registros con relaciones inválidas. La base no puede utilizarse hasta recuperar una copia válida.")
}
fn row_json(row: &Row<'_>) -> rusqlite::Result<Value> {
    let mut result = Map::new();
    for i in 0..row.as_ref().column_count() {
        let key = row.as_ref().column_name(i)?;
        let mut chars = key.chars();
        let key = chars.next().unwrap_or('_').to_lowercase().to_string() + chars.as_str();
        let value = match row.get_ref(i)? {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(n) if ["active","cancelled","automaticBackups","darkMode","reducedMotion","successful","discarded","wholesale","locked"].contains(&key.as_str()) => json!(n != 0),
            ValueRef::Integer(n) => json!(n), ValueRef::Real(n) => json!(n),
            ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
            ValueRef::Blob(b) => json!(STANDARD.encode(b)),
        };
        result.insert(key, value);
    }
    Ok(Value::Object(result))
}
pub fn all(db: &Connection, sql: &str, params: impl Params) -> Result<Vec<Value>> { Ok(db.prepare(sql)?.query_map(params, row_json)?.collect::<std::result::Result<_,_>>()?) }
pub fn one(db: &Connection, sql: &str, params: impl Params) -> Result<Value> { all(db,sql,params)?.into_iter().next().ok_or_else(||AppError::rule("NOT_FOUND","No encontramos el registro. Actualiza la lista e intenta nuevamente.")) }
pub fn scalar(db: &Connection, sql: &str, params: impl Params) -> Result<i64> { Ok(db.query_row(sql,params,|r| r.get(0))?) }
pub fn audit(db: &Connection, now: &str, user: Option<i64>, action: &str, reference: &str, detail: &str) -> Result<()> {
    db.execute("INSERT INTO AuditEvent(CreatedAt,UserId,Action,Reference,Detail) VALUES (?1,?2,?3,?4,?5)",rusqlite::params![now,user,action,reference,detail])?; Ok(())
}
pub fn folio(db: &Connection, key: &str, prefix: &str) -> Result<String> {
    db.execute("INSERT INTO Sequence(Key,Value) VALUES (?1,1) ON CONFLICT(Key) DO UPDATE SET Value=Value+1",[key])?;
    Ok(format!("{}-{:06}",prefix,scalar(db,"SELECT Value FROM Sequence WHERE Key=?1",[key])?))
}
