//! Synchronous storage boundary. Business services share parameterized queries;
//! SQLite is retained for legacy data and portable, verified backup containers.
use crate::error::{AppError, Result, ensure};
use mysql::prelude::Queryable;
use rusqlite::types::{FromSql, ToSql, ToSqlOutput, Value, ValueRef};
use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    path::Path,
};

pub use rusqlite::{OpenFlags, TransactionBehavior};

pub trait Params {
    fn values(self) -> rusqlite::Result<Vec<Value>>;
}
fn owned(value: &dyn ToSql) -> rusqlite::Result<Value> {
    match value.to_sql()? {
        ToSqlOutput::Borrowed(v) => Ok(v.into()),
        ToSqlOutput::Owned(v) => Ok(v),
        _ => Err(rusqlite::Error::InvalidParameterName(
            "Unsupported SQL value".into(),
        )),
    }
}
impl Params for [(); 0] {
    fn values(self) -> rusqlite::Result<Vec<Value>> {
        Ok(Vec::new())
    }
}
impl Params for &[&dyn ToSql] {
    fn values(self) -> rusqlite::Result<Vec<Value>> {
        self.iter().map(|v| owned(*v)).collect()
    }
}
impl Params for Vec<Value> {
    fn values(self) -> rusqlite::Result<Vec<Value>> {
        Ok(self)
    }
}
macro_rules! arrays {
    ($($n:literal),*) => {$ (
        impl<T: ToSql> Params for [T; $n] {
            fn values(self) -> rusqlite::Result<Vec<Value>> {
                self.iter().map(|v| owned(v)).collect()
            }
        }
    )*};
}
arrays!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32
);

#[derive(Clone, Debug)]
pub struct Row {
    pub columns: Vec<String>,
    pub values: Vec<Value>,
}
pub trait Index {
    fn index(&self, row: &Row) -> rusqlite::Result<usize>;
}
impl Index for usize {
    fn index(&self, row: &Row) -> rusqlite::Result<usize> {
        if *self < row.values.len() {
            Ok(*self)
        } else {
            Err(rusqlite::Error::InvalidColumnIndex(*self))
        }
    }
}
impl Index for &str {
    fn index(&self, row: &Row) -> rusqlite::Result<usize> {
        row.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(self))
            .ok_or_else(|| rusqlite::Error::InvalidColumnName(self.to_string()))
    }
}
impl Row {
    pub fn get<I: Index, T: FromSql>(&self, index: I) -> rusqlite::Result<T> {
        let i = index.index(self)?;
        T::column_result(ValueRef::from(&self.values[i])).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(i, self.values[i].data_type(), Box::new(e))
        })
    }
    pub fn get_ref(&self, i: usize) -> rusqlite::Result<ValueRef<'_>> {
        i.index(self)?;
        Ok(ValueRef::from(&self.values[i]))
    }
    pub fn as_ref(&self) -> &Self {
        self
    }
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
    pub fn column_name(&self, i: usize) -> rusqlite::Result<&str> {
        self.columns
            .get(i)
            .map(String::as_str)
            .ok_or(rusqlite::Error::InvalidColumnIndex(i))
    }
}

enum Backend {
    Sqlite(rusqlite::Connection),
    MySql {
        conn: RefCell<mysql::Conn>,
        transaction: Cell<bool>,
        poisoned: Cell<bool>,
    },
}
pub struct Connection {
    backend: Backend,
}
impl From<rusqlite::Connection> for Connection {
    fn from(connection: rusqlite::Connection) -> Self {
        Self {
            backend: Backend::Sqlite(connection),
        }
    }
}
impl Connection {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(rusqlite::Connection::open(path)?.into())
    }
    pub fn open_in_memory() -> Result<Self> {
        Ok(rusqlite::Connection::open_in_memory()?.into())
    }
    pub fn open_with_flags(path: &Path, flags: OpenFlags) -> Result<Self> {
        Ok(rusqlite::Connection::open_with_flags(path, flags)?.into())
    }
    pub fn sqlite(&self) -> Result<&rusqlite::Connection> {
        match &self.backend {
            Backend::Sqlite(db) => Ok(db),
            _ => Err(AppError::rule(
                "DATABASE_BACKEND",
                "Esta operación requiere una copia SQLite.",
            )),
        }
    }
    pub fn sqlite_mut(&mut self) -> Result<&mut rusqlite::Connection> {
        match &mut self.backend {
            Backend::Sqlite(db) => Ok(db),
            _ => Err(AppError::rule(
                "DATABASE_BACKEND",
                "Esta operación requiere una copia SQLite.",
            )),
        }
    }
    pub fn is_mysql(&self) -> bool {
        matches!(self.backend, Backend::MySql { .. })
    }
    pub fn from_mysql(mut conn: mysql::Conn) -> Result<Self> {
        conn.query_drop("SET SESSION sql_mode='STRICT_ALL_TABLES,NO_ZERO_DATE,NO_ZERO_IN_DATE,NO_AUTO_VALUE_ON_ZERO,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION,ANSI_QUOTES,PIPES_AS_CONCAT,NO_BACKSLASH_ESCAPES'")?;
        conn.query_drop("SET NAMES utf8mb4 COLLATE utf8mb4_0900_bin")?;
        conn.query_drop("SET SESSION innodb_lock_wait_timeout=10")?;
        Ok(Self {
            backend: Backend::MySql {
                conn: RefCell::new(conn),
                transaction: Cell::new(false),
                poisoned: Cell::new(false),
            },
        })
    }
    pub fn prepare<'a>(&'a self, sql: &str) -> Result<Statement<'a>> {
        Ok(Statement {
            db: self,
            sql: sql.to_owned(),
        })
    }
    pub fn rows(&self, sql: &str, params: impl Params) -> Result<Vec<Row>> {
        self.healthy()?;
        let values = params.values()?;
        match &self.backend {
            Backend::Sqlite(db) => {
                let mut statement = db.prepare(sql)?;
                let columns = statement
                    .column_names()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                let count = columns.len();
                let rows = statement
                    .query_map(rusqlite::params_from_iter(values), |r| {
                        Ok(Row {
                            columns: columns.clone(),
                            values: (0..count)
                                .map(|i| r.get::<_, Value>(i))
                                .collect::<rusqlite::Result<_>>()?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            }
            Backend::MySql { conn, .. } => {
                let (query, bindings) = crate::mysql_dialect::translate(sql, &values)?;
                let rows: Vec<mysql::Row> = conn.borrow_mut().exec(query, bindings)?;
                rows.into_iter().map(mysql_row).collect()
            }
        }
    }
    pub fn query_row<T>(
        &self,
        sql: &str,
        params: impl Params,
        f: impl FnOnce(&Row) -> rusqlite::Result<T>,
    ) -> Result<T> {
        let rows = self.rows(sql, params)?;
        let row = rows
            .first()
            .ok_or_else(|| AppError::from(rusqlite::Error::QueryReturnedNoRows))?;
        Ok(f(row)?)
    }
    pub fn execute(&self, sql: &str, params: impl Params) -> Result<usize> {
        self.healthy()?;
        let values = params.values()?;
        match &self.backend {
            Backend::Sqlite(db) => Ok(db.execute(sql, rusqlite::params_from_iter(values))?),
            Backend::MySql { conn, .. } => {
                let (query, bindings) = crate::mysql_dialect::translate(sql, &values)?;
                let mut conn = conn.borrow_mut();
                conn.exec_drop(query, bindings)?;
                Ok(conn.affected_rows() as usize)
            }
        }
    }
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.healthy()?;
        match &self.backend {
            Backend::Sqlite(db) => Ok(db.execute_batch(sql)?),
            Backend::MySql { conn, .. } => {
                for statement in crate::mysql_dialect::statements(sql)? {
                    let (query, _) = crate::mysql_dialect::translate(&statement, &[])?;
                    conn.borrow_mut().query_drop(query)?;
                }
                Ok(())
            }
        }
    }
    pub(crate) fn mysql_ddl(&self, sql: &str) -> Result<()> {
        match &self.backend {
            Backend::MySql { conn, .. } => {
                for statement in crate::mysql_dialect::statements(sql)? {
                    conn.borrow_mut().query_drop(statement)?;
                }
                Ok(())
            }
            _ => Err(AppError::rule("DATABASE_BACKEND", "Se requiere MySQL.")),
        }
    }
    pub fn last_insert_rowid(&self) -> i64 {
        match &self.backend {
            Backend::Sqlite(db) => db.last_insert_rowid(),
            Backend::MySql { conn, .. } => conn.borrow().last_insert_id() as i64,
        }
    }
    pub fn is_autocommit(&self) -> bool {
        match &self.backend {
            Backend::Sqlite(db) => db.is_autocommit(),
            Backend::MySql { transaction, .. } => !transaction.get(),
        }
    }
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        self.begin(TransactionBehavior::Immediate)
    }
    pub fn transaction_with_behavior(
        &mut self,
        behavior: TransactionBehavior,
    ) -> Result<Transaction<'_>> {
        self.begin(behavior)
    }
    pub fn unchecked_transaction(&self) -> Result<Transaction<'_>> {
        self.begin(TransactionBehavior::Deferred)
    }
    fn begin(&self, behavior: TransactionBehavior) -> Result<Transaction<'_>> {
        self.healthy()?;
        ensure(
            self.is_autocommit(),
            "DATABASE_TRANSACTION",
            "Ya hay una operación de datos en curso.",
        )?;
        match &self.backend {
            Backend::Sqlite(db) => db.execute_batch(match behavior {
                TransactionBehavior::Deferred => "BEGIN DEFERRED",
                TransactionBehavior::Exclusive => "BEGIN EXCLUSIVE",
                _ => "BEGIN IMMEDIATE",
            })?,
            Backend::MySql {
                conn, transaction, ..
            } => {
                let mut conn = conn.borrow_mut();
                conn.query_drop("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")?;
                conn.query_drop("START TRANSACTION")?;
                transaction.set(true);
            }
        }
        Ok(Transaction {
            db: self,
            completed: false,
        })
    }
    fn finish(&self, commit: bool) -> Result<()> {
        let sql = if commit { "COMMIT" } else { "ROLLBACK" };
        match &self.backend {
            Backend::Sqlite(db) => db.execute_batch(sql)?,
            Backend::MySql {
                conn,
                transaction,
                poisoned,
            } => {
                if let Err(error) = conn.borrow_mut().query_drop(sql) {
                    poisoned.set(true);
                    return Err(error.into());
                }
                transaction.set(false);
            }
        }
        Ok(())
    }
    fn healthy(&self) -> Result<()> {
        ensure(
            !matches!(&self.backend,Backend::MySql{poisoned,..} if poisoned.get()),
            "STORAGE_UNAVAILABLE",
            "Se perdió la confirmación de una operación. Cierra y abre Donovan POS Pro para consultar su estado antes de reintentar; conserva la referencia de la venta.",
        )
    }
}
pub struct Statement<'a> {
    db: &'a Connection,
    sql: String,
}
impl Statement<'_> {
    pub fn query(&self, params: impl Params) -> Result<Rows> {
        Ok(Rows {
            rows: self.db.rows(&self.sql, params)?.into_iter(),
        })
    }
    pub fn query_map<T>(
        &self,
        params: impl Params,
        mut f: impl FnMut(&Row) -> rusqlite::Result<T>,
    ) -> Result<std::vec::IntoIter<rusqlite::Result<T>>> {
        Ok(self
            .db
            .rows(&self.sql, params)?
            .iter()
            .map(&mut f)
            .collect::<Vec<_>>()
            .into_iter())
    }
}
pub struct Rows {
    rows: std::vec::IntoIter<Row>,
}
impl Rows {
    pub fn next(&mut self) -> Result<Option<Row>> {
        Ok(self.rows.next())
    }
}
pub struct Transaction<'a> {
    db: &'a Connection,
    completed: bool,
}
impl Deref for Transaction<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.db
    }
}
impl Transaction<'_> {
    pub fn commit(mut self) -> Result<()> {
        self.db.finish(true)?;
        self.completed = true;
        Ok(())
    }
}
impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.db.finish(false);
        }
    }
}

fn mysql_row(row: mysql::Row) -> Result<Row> {
    use mysql::{Value as M, consts::ColumnType as C};
    let columns = row
        .columns_ref()
        .iter()
        .map(|c| c.name_str().to_string())
        .collect();
    let mut values = Vec::with_capacity(row.len());
    for (i, column) in row.columns_ref().iter().enumerate() {
        let value = match row.as_ref(i).ok_or_else(|| {
            AppError::rule("DATABASE_VALUE", "La base devolvió un valor incompleto.")
        })? {
            M::NULL => Value::Null,
            M::Int(n) => Value::Integer(*n),
            M::UInt(n) => Value::Integer(i64::try_from(*n).map_err(|_| {
                AppError::rule("DATABASE_VALUE", "Un importe excede el límite admitido.")
            })?),
            M::Float(n) => Value::Real(*n as f64),
            M::Double(n) => Value::Real(*n),
            M::Bytes(b)
                if matches!(
                    column.column_type(),
                    C::MYSQL_TYPE_DECIMAL | C::MYSQL_TYPE_NEWDECIMAL
                ) =>
            {
                let s = std::str::from_utf8(b)
                    .map_err(|_| AppError::rule("DATABASE_VALUE", "Importe no válido."))?;
                if let Ok(n) = s.parse::<i64>() {
                    Value::Integer(n)
                } else {
                    Value::Real(
                        s.parse::<f64>()
                            .map_err(|_| AppError::rule("DATABASE_VALUE", "Importe no válido."))?,
                    )
                }
            }
            M::Bytes(b) if column.character_set() == 63 => Value::Blob(b.clone()),
            M::Bytes(b) => Value::Text(
                String::from_utf8(b.clone())
                    .map_err(|_| AppError::rule("DATABASE_VALUE", "Texto no válido en la base."))?,
            ),
            _ => {
                return Err(AppError::rule(
                    "DATABASE_VALUE",
                    "La base contiene un tipo de fecha no compatible con esta versión.",
                ));
            }
        };
        values.push(value);
    }
    Ok(Row { columns, values })
}
