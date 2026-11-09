//! Real InnoDB storage and a lossless bridge to versioned portable backups.
//! A migration always targets a new schema; the source is never overwritten.
use crate::{
    database::{Connection, Row},
    db,
    error::{Result, ensure},
    storage,
};
use rusqlite::types::Value;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, path::Path};

pub const DDL: &str = include_str!("../migrations/mysql/001_mysql.sql");
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
    indexes: Vec<Index>,
    checks: Vec<String>,
}
#[derive(Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(rename = "type")]
    kind: String,
    nullable: bool,
    primary: bool,
    auto: bool,
}
#[derive(Deserialize)]
pub struct ForeignKey {
    table: String,
    from: String,
    to: String,
    delete: String,
    update: String,
}
#[derive(Deserialize)]
struct Index {
    name: String,
    unique: bool,
    columns: Vec<String>,
}
pub fn contract() -> Result<Vec<Table>> {
    Ok(serde_json::from_str(include_str!(
        "../migrations/mysql/contract.json"
    ))?)
}
pub fn identifier(name: &str) -> Result<String> {
    ensure(
        !name.is_empty()
            && name.len() <= 64
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'),
        "DATABASE_IDENTIFIER",
        "Identificador de base no válido.",
    )?;
    Ok(format!("`{name}`"))
}
fn digest() -> String {
    format!("{:x}", Sha256::digest(DDL.as_bytes()))
}
pub fn initialize(target: &mut Connection, now: &str) -> Result<()> {
    ensure(target.is_mysql(), "DATABASE_BACKEND", "Se requiere MySQL.")?;
    ensure(
        db::scalar(
            target,
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema=DATABASE()",
            [],
        )? == 0,
        "DATABASE_EXISTS",
        "El esquema de destino ya contiene tablas. No se ha sobrescrito.",
    )?;
    target.mysql_ddl(DDL)?;
    let mut seed = Connection::open_in_memory()?;
    db::migrate(&mut seed, now)?;
    copy(&seed, target)?;
    compatible(target)?;
    Ok(())
}
pub fn compatible(connection: &Connection) -> Result<i64> {
    ensure(
        connection.is_mysql(),
        "DATABASE_BACKEND",
        "Se requiere MySQL.",
    )?;
    let version: String = connection.query_row("SELECT VERSION()", [], |r| r.get(0))?;
    ensure(
        version.starts_with("8.4."),
        "MYSQL_VERSION",
        "Esta edición requiere MySQL 8.4 LTS. Conserva los datos y utiliza el motor incluido.",
    )?;
    let tables = contract()?;
    let expected: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("../migrations/mysql/schema-hashes.json"))?;
    ensure(
        schema_hashes(connection)? == expected,
        "INCOMPATIBLE_DATABASE",
        "Las tablas, índices o reglas de MySQL fueron modificados. La base no se ha activado.",
    )?;
    ensure(
        db::scalar(
            connection,
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema=DATABASE()",
            [],
        )? == tables.len() as i64,
        "INCOMPATIBLE_DATABASE",
        "El esquema MySQL contiene tablas no reconocidas o incompletas.",
    )?;
    ensure(
        db::scalar(
            connection,
            "SELECT COUNT(*) FROM information_schema.triggers WHERE trigger_schema=DATABASE()",
            [],
        )? == 0,
        "INCOMPATIBLE_DATABASE",
        "El esquema contiene reglas externas no reconocidas.",
    )?;
    for table in &tables {
        let columns=connection.rows("SELECT COLUMN_NAME,COLUMN_TYPE,IS_NULLABLE,COLUMN_KEY,EXTRA FROM information_schema.columns WHERE table_schema=DATABASE() AND table_name=?1 ORDER BY ORDINAL_POSITION",[table.name.as_str()])?;
        ensure(
            columns.len() == table.columns.len(),
            "INCOMPATIBLE_DATABASE",
            "Las columnas de MySQL no corresponden a esta versión.",
        )?;
        for (actual, expected) in columns.iter().zip(&table.columns) {
            let extra: String = actual.get(4)?;
            ensure(
                actual.get::<_, String>(0)? == expected.name
                    && actual.get::<_, String>(1)? == expected.kind
                    && (actual.get::<_, String>(2)? == "YES") == expected.nullable
                    && (actual.get::<_, String>(3)? == "PRI") == expected.primary
                    && extra.contains("auto_increment") == expected.auto,
                "INCOMPATIBLE_DATABASE",
                "Un tipo, clave o columna MySQL fue modificado.",
            )?;
        }
        let engine:String=connection.query_row("SELECT ENGINE FROM information_schema.tables WHERE table_schema=DATABASE() AND table_name=?1",[table.name.as_str()],|r|r.get(0))?;
        ensure(
            engine == "InnoDB",
            "INCOMPATIBLE_DATABASE",
            "La base debe conservar transacciones InnoDB.",
        )?;
        let fks=connection.rows("SELECT k.REFERENCED_TABLE_NAME,k.COLUMN_NAME,k.REFERENCED_COLUMN_NAME,r.DELETE_RULE,r.UPDATE_RULE FROM information_schema.key_column_usage k JOIN information_schema.referential_constraints r ON r.CONSTRAINT_SCHEMA=k.CONSTRAINT_SCHEMA AND r.CONSTRAINT_NAME=k.CONSTRAINT_NAME WHERE k.TABLE_SCHEMA=DATABASE() AND k.TABLE_NAME=?1 AND k.REFERENCED_TABLE_NAME IS NOT NULL",[table.name.as_str()])?;
        ensure(
            fks.len() == table.foreign_keys.len(),
            "INCOMPATIBLE_DATABASE",
            "Faltan relaciones en MySQL.",
        )?;
        for expected in &table.foreign_keys {
            ensure(
                fks.iter().any(|r| {
                    r.get::<_, String>(0)
                        .ok()
                        .is_some_and(|s| s.eq_ignore_ascii_case(&expected.table))
                        && r.get::<_, String>(1).ok().as_deref() == Some(&expected.from)
                        && r.get::<_, String>(2).ok().as_deref() == Some(&expected.to)
                        && r.get::<_, String>(3).ok().as_deref() == Some(expected.delete.as_str())
                        && r.get::<_, String>(4).ok().as_deref() == Some(expected.update.as_str())
                }),
                "INCOMPATIBLE_DATABASE",
                "Las relaciones MySQL fueron modificadas.",
            )?;
        }
        for index in &table.indexes {
            let actual=connection.rows("SELECT COLUMN_NAME,NON_UNIQUE FROM information_schema.statistics WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME=?1 AND INDEX_NAME=?2 ORDER BY SEQ_IN_INDEX",rusqlite::params![table.name,index.name])?;
            ensure(
                actual.len() == index.columns.len()
                    && actual.iter().zip(&index.columns).all(|(r, c)| {
                        r.get::<_, String>(0).ok().as_ref() == Some(c)
                            && r.get::<_, i64>(1).ok() == Some(if index.unique { 0 } else { 1 })
                    }),
                "INCOMPATIBLE_DATABASE",
                "Un índice de negocio MySQL no es compatible.",
            )?;
        }
        ensure(
            db::scalar(
                connection,
                "SELECT COUNT(*) FROM information_schema.table_constraints WHERE CONSTRAINT_SCHEMA=DATABASE() AND TABLE_NAME=?1 AND CONSTRAINT_TYPE='CHECK' AND ENFORCED='YES'",
                [table.name.as_str()],
            )? == table.checks.len() as i64,
            "INCOMPATIBLE_DATABASE",
            "Faltan validaciones obligatorias en MySQL.",
        )?;
    }
    ensure(
        db::scalar(
            connection,
            "SELECT COUNT(*) FROM information_schema.statistics WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME='CashSession' AND INDEX_NAME='IX_CashSession_OneOpen' AND NON_UNIQUE=0 AND EXPRESSION IS NOT NULL",
            [],
        )? == 1,
        "INCOMPATIBLE_DATABASE",
        "Falta la protección de caja abierta.",
    )?;
    ensure(
        connection.query_row(
            "SELECT Value FROM AppMeta WHERE Key='mysqlSchemaChecksum'",
            [],
            |r| r.get::<_, String>(0),
        )? == digest(),
        "INCOMPATIBLE_DATABASE",
        "La migración MySQL no está reconocida.",
    )?;
    ensure(
        connection.query_row("SELECT Value FROM AppMeta WHERE Key='product'", [], |r| {
            r.get::<_, String>(0)
        })? == "donovan-pos",
        "INCOMPATIBLE_DATABASE",
        "La base pertenece a otro producto.",
    )?;
    let migrations = connection.rows("SELECT Version FROM NativeMigration ORDER BY Version", [])?;
    ensure(
        migrations
            .iter()
            .map(|r| r.get::<_, i64>(0))
            .collect::<rusqlite::Result<Vec<_>>>()?
            == (2..=db::SCHEMA_VERSION).collect::<Vec<_>>(),
        "INCOMPATIBLE_DATABASE",
        "Historial de migraciones incompleto.",
    )?;
    integrity(connection)?;
    Ok(db::SCHEMA_VERSION)
}
pub fn schema_hashes(
    connection: &Connection,
) -> Result<std::collections::BTreeMap<String, String>> {
    let mut hashes = std::collections::BTreeMap::new();
    for table in contract()? {
        let create: String = connection.query_row(
            &format!("SHOW CREATE TABLE {}", identifier(&table.name)?),
            [],
            |r| r.get(1),
        )?;
        let normalized = create
            .split_whitespace()
            .filter(|s| !s.starts_with("AUTO_INCREMENT="))
            .collect::<Vec<_>>()
            .join(" ");
        hashes.insert(
            table.name,
            format!("{:x}", Sha256::digest(normalized.as_bytes())),
        );
    }
    Ok(hashes)
}
pub fn integrity(connection: &Connection) -> Result<()> {
    let transaction = if connection.is_autocommit() {
        Some(connection.unchecked_transaction()?)
    } else {
        None
    };
    for table in contract()? {
        for fk in table.foreign_keys {
            let sql = format!(
                "SELECT COUNT(*) FROM {} c LEFT JOIN {} p ON c.{}=p.{} WHERE c.{} IS NOT NULL AND p.{} IS NULL",
                identifier(&table.name)?,
                identifier(&fk.table)?,
                identifier(&fk.from)?,
                identifier(&fk.to)?,
                identifier(&fk.from)?,
                identifier(&fk.to)?
            );
            ensure(
                db::scalar(connection, &sql, [])? == 0,
                "INCONSISTENT_DATA",
                "MySQL contiene relaciones inválidas. Restaura un respaldo verificado.",
            )?;
        }
    }
    db::business_integrity(connection)?;
    if let Some(tx) = transaction {
        tx.commit()?;
    }
    Ok(())
}
fn row_digest(hash: &mut Sha256, row: &Row) {
    for value in &row.values {
        let (kind, bytes) = match value {
            Value::Null => (0, Vec::new()),
            Value::Integer(v) => (1, v.to_le_bytes().to_vec()),
            Value::Real(v) => (2, v.to_bits().to_le_bytes().to_vec()),
            Value::Text(v) => (3, v.as_bytes().to_vec()),
            Value::Blob(v) => (4, v.clone()),
        };
        hash.update([kind]);
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
}
/// Both connections are held in transactions throughout the data comparison.
pub fn copy(source: &Connection, target: &mut Connection) -> Result<()> {
    let source_tx = source.unchecked_transaction()?;
    let target_tx = target.transaction()?;
    let tables = contract()?;
    for table in tables.iter().rev() {
        target_tx.execute(&format!("DELETE FROM {}", identifier(&table.name)?), [])?;
    }
    for table in &tables {
        let columns = table
            .columns
            .iter()
            .map(|c| identifier(&c.name))
            .collect::<Result<Vec<_>>>()?
            .join(",");
        let keys = table
            .columns
            .iter()
            .filter(|c| c.primary)
            .map(|c| identifier(&c.name))
            .collect::<Result<Vec<_>>>()?
            .join(",");
        let query = format!(
            "SELECT {columns} FROM {} ORDER BY {keys} LIMIT 128 OFFSET ?1",
            identifier(&table.name)?
        );
        let insert = format!(
            "INSERT INTO {} ({columns}) VALUES ({})",
            identifier(&table.name)?,
            (1..=table.columns.len())
                .map(|i| format!("?{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut offset = 0i64;
        let mut source_hash = Sha256::new();
        loop {
            let rows = source_tx.rows(&query, [offset])?;
            if rows.is_empty() {
                break;
            }
            offset += rows.len() as i64;
            for row in rows {
                row_digest(&mut source_hash, &row);
                target_tx.execute(&insert, row.values)?;
            }
        }
        let mut target_hash = Sha256::new();
        let mut count = 0i64;
        loop {
            let rows = target_tx.rows(&query, [count])?;
            if rows.is_empty() {
                break;
            }
            count += rows.len() as i64;
            for row in rows {
                row_digest(&mut target_hash, &row);
            }
        }
        ensure(
            offset == count && source_hash.finalize() == target_hash.finalize(),
            "MIGRATION_MISMATCH",
            "La copia no coincide con el original. No se ha activado la nueva base.",
        )?;
    }
    if target_tx.is_mysql() {
        target_tx.execute("INSERT INTO AppMeta(Key,Value) VALUES('mysqlSchemaChecksum',?1) ON CONFLICT(Key) DO UPDATE SET Value=excluded.Value",[digest()])?;
    }
    db::integrity(&target_tx)?;
    target_tx.commit()?;
    source_tx.commit()?;
    Ok(())
}
pub fn snapshot(source: &Connection, target: &Path) -> Result<()> {
    let size = db::scalar(
        source,
        "SELECT COALESCE(SUM(DATA_LENGTH+INDEX_LENGTH),0) FROM information_schema.tables WHERE table_schema=DATABASE()",
        [],
    )? as u64;
    ensure(
        size <= storage::MAX_DATABASE_BYTES,
        "FILE_TOO_LARGE",
        "La base supera el límite de respaldo de 8 GB.",
    )?;
    storage::require_space(
        target.parent().unwrap_or(Path::new(".")),
        size.saturating_mul(2),
    )?;
    drop(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?,
    );
    let mut destination = db::connect(target, false)?;
    db::migrate(&mut destination, &chrono::Utc::now().to_rfc3339())?;
    copy(source, &mut destination)?;
    db::compatible(&destination)?;
    destination.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
    ensure(
        std::fs::metadata(target)?.len() <= storage::MAX_DATABASE_BYTES,
        "FILE_TOO_LARGE",
        "El respaldo supera el límite de 8 GB.",
    )?;
    Ok(())
}
