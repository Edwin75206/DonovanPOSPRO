use crate::database::Connection;
use crate::{
    auth, db,
    error::{AppError, Result, ensure},
    models::Actor,
    secure,
};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::Arc,
};

pub trait Clock: Send + Sync {
    fn timestamp(&self) -> i64;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn timestamp(&self) -> i64 {
        Utc::now().timestamp()
    }
}
pub fn stamp(time: i64) -> String {
    DateTime::from_timestamp(time, 0)
        .unwrap_or_default()
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
pub struct Engine {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub(crate) connection: Option<Connection>,
    pub(crate) session_token: Option<String>,
    pub(crate) clock: Arc<dyn Clock>,
    pub demo: bool,
    pub(crate) demo_password: Option<String>,
    pub(crate) _lock: File,
    pub last_integrity: Option<String>,
    pub startup_error: Option<AppError>,
    pub(crate) recovery: crate::recovery::RecoveryState,
    pub(crate) importing: crate::importing::ImportState,
    pub(crate) mysql_runtime: Option<PathBuf>,
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.connection.take();
        if self.mysql_runtime.is_some() && self.root.join("mysql/instance.json").exists() {
            if let Err(error) = crate::mysql_runtime::shutdown(&self.root) {
                self.log_error("storage", "mysql-shutdown", &error);
            }
        }
    }
}
impl Engine {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        Self::with_clock(root, Arc::new(SystemClock))
    }
    pub fn with_clock(root: impl AsRef<Path>, clock: Arc<dyn Clock>) -> Result<Self> {
        Self::open_internal(root, clock, None)
    }
    pub fn open_desktop(root: impl AsRef<Path>, runtime: PathBuf) -> Result<Self> {
        Self::open_internal(root, Arc::new(SystemClock), Some(runtime))
    }
    pub fn with_mysql_clock(
        root: impl AsRef<Path>,
        clock: Arc<dyn Clock>,
        runtime: PathBuf,
    ) -> Result<Self> {
        Self::open_internal(root, clock, Some(runtime))
    }
    fn open_internal(
        root: impl AsRef<Path>,
        clock: Arc<dyn Clock>,
        runtime: Option<PathBuf>,
    ) -> Result<Self> {
        std::fs::create_dir_all(root.as_ref())?;
        let root = root.as_ref().canonicalize()?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(".instance.lock"))?;
        lock.try_lock_exclusive().map_err(|_| {
            AppError::rule(
                "ALREADY_OPEN",
                "Este negocio ya está abierto en otra ventana de Donovan POS Pro.",
            )
        })?;
        let db_path = root.join("donovan.db");
        let mut engine = Self {
            root,
            db_path,
            connection: None,
            session_token: None,
            clock,
            demo: false,
            demo_password: None,
            _lock: lock,
            last_integrity: None,
            startup_error: None,
            recovery: Default::default(),
            importing: Default::default(),
            mysql_runtime: runtime,
        };
        match engine.open_database() {
            Ok(()) => {}
            Err(error) => engine.startup_error = Some(error),
        };
        if engine.connection.is_some() {
            engine.session_token = secure::load(&engine.session_path()).ok().flatten();
            if let Err(error) = engine.actor("sell") {
                if error.code != "SESSION_LOCKED" {
                    engine.session_token = None;
                }
            }
        }
        Ok(engine)
    }
    pub(crate) fn open_database(&mut self) -> Result<()> {
        let active = self.root.join("active.json");
        if active.exists() {
            let config: Value = serde_json::from_slice(&std::fs::read(&active)?)?;
            if config["version"] == 2 && config["backend"] == "mysql" {
                let schema = config["database"]
                    .as_str()
                    .ok_or_else(|| AppError::rule("MYSQL_CONFIG", "Falta el esquema activo."))?;
                crate::mysql_runtime::validate_schema(schema)?;
                let runtime = self.mysql_runtime.as_ref().ok_or_else(|| {
                    AppError::rule(
                        "MYSQL_RUNTIME_MISSING",
                        "Abre esta base con el instalador completo de Donovan POS Pro.",
                    )
                })?;
                crate::mysql_runtime::ensure_running(&self.root, runtime)?;
                let connection = crate::mysql_runtime::connect(&self.root, schema)?;
                db::compatible(&connection)?;
                self.db_path = self.root.join("mysql").join(schema);
                self.connection = Some(connection);
                self.startup_error = None;
                self.last_integrity = Some(self.now_string());
                return Ok(());
            }
        } else if !self.root.join("donovan.db").exists() {
            if let Some(runtime) = &self.mysql_runtime {
                crate::mysql_runtime::ensure_running(&self.root, runtime)?;
                let (schema, connection) =
                    crate::mysql_runtime::create_schema(&self.root, None, &self.now_string())?;
                crate::storage::write_mysql_active(&self.root, &schema, false)?;
                self.db_path = self.root.join("mysql").join(&schema);
                self.connection = Some(connection);
                self.startup_error = None;
                self.last_integrity = Some(self.now_string());
                return Ok(());
            }
        }
        let configured = match std::fs::read(self.root.join("active.json")) {
            Ok(raw) => {
                let config: Value = serde_json::from_slice(&raw)?;
                ensure(
                    config["version"] == 1,
                    "INVALID_DATA",
                    "La configuración pertenece a una versión no compatible.",
                )?;
                let relative = PathBuf::from(config["database"].as_str().ok_or_else(|| {
                    AppError::rule(
                        "INVALID_DATA",
                        "La ubicación configurada no es válida. Restaura una copia verificada.",
                    )
                })?);
                let parts: Vec<_> = relative.components().collect();
                let managed = parts.len() == 3
                    && parts[0].as_os_str() == "businesses"
                    && uuid::Uuid::parse_str(parts[1].as_os_str().to_str().unwrap_or("")).is_ok()
                    && parts[2].as_os_str() == "donovan.db";
                ensure(
                    relative == Path::new("donovan.db") || managed,
                    "INVALID_DATA",
                    "La base debe permanecer en una carpeta administrada de Donovan POS Pro.",
                )?;
                self.db_path = self.root.join(relative);
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if configured {
            ensure(
                self.db_path.try_exists()?,
                "STORAGE_MISSING",
                "No encontramos la base configurada. Conserva tus archivos y restaura un respaldo; no se creó una base vacía en su lugar.",
            )?;
        }
        let existed = self.db_path.try_exists()?;
        if !existed && std::fs::symlink_metadata(&self.db_path).is_ok() {
            return Err(AppError::rule(
                "INVALID_DATA",
                "La ubicación de la base no es un archivo local válido.",
            ));
        }
        if existed {
            ensure(
                self.db_path.canonicalize()?.starts_with(&self.root),
                "INVALID_DATA",
                "La base configurada está fuera de la carpeta persistente de la aplicación.",
            )?;
        }
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Validate a pre-existing file read-only before enabling WAL or migrating anything.
        if existed {
            let original = db::connect(&self.db_path, true)?;
            db::compatible(&original)?;
            if db::schema(&original)? < db::SCHEMA_VERSION {
                let safety = self
                    .root
                    .join(format!("before-migration-{}.sqlite", uuid::Uuid::new_v4()));
                let mut target = Connection::open(safety)?;
                rusqlite::backup::Backup::new(original.sqlite()?, target.sqlite_mut()?)?
                    .run_to_completion(256, std::time::Duration::from_millis(10), None)?;
            }
        }
        let mut connection = db::connect(&self.db_path, false)?;
        db::migrate(&mut connection, &self.now_string())?;
        if !configured {
            crate::storage::write_active(&self.root, Path::new("donovan.db"), false)?;
        }
        self.connection = Some(connection);
        self.startup_error = None;
        self.last_integrity = Some(self.now_string());
        Ok(())
    }
    pub fn db(&self) -> Result<&Connection> {
        self.connection.as_ref().ok_or_else(|| {
            self.startup_error.clone().unwrap_or_else(|| {
                AppError::rule(
                    "STORAGE_UNAVAILABLE",
                    "No podemos acceder a la información del negocio.",
                )
            })
        })
    }
    pub(crate) fn db_mut(&mut self) -> Result<&mut Connection> {
        self.connection.as_mut().ok_or_else(|| {
            AppError::rule(
                "STORAGE_UNAVAILABLE",
                "No podemos acceder a la información del negocio.",
            )
        })
    }
    pub fn now(&self) -> i64 {
        self.clock.timestamp()
    }
    pub fn now_string(&self) -> String {
        stamp(self.now())
    }
    pub(crate) fn session_path(&self) -> PathBuf {
        self.root.join("session.dpapi")
    }
    pub fn actor(&self, permission: &str) -> Result<Actor> {
        let token = self.session_token.as_ref().ok_or_else(|| {
            AppError::rule(
                "SESSION_EXPIRED",
                "Tu sesión terminó. Inicia sesión para continuar.",
            )
        })?;
        let row=db::one(self.db()?,"SELECT u.Id,u.Username,u.Name,u.Role,s.IssuedAt,s.ExpiresAt,s.LastSeenAt,s.Locked FROM UserSession s JOIN User u ON u.Id=s.UserId WHERE s.TokenHash=?1 AND u.Active=1 AND u.AuthVersion=s.AuthVersion",[auth::digest(token)]).map_err(|error|if error.code=="NOT_FOUND"{AppError::rule("SESSION_EXPIRED","Tu sesión terminó. Inicia sesión para continuar.")}else{error})?;
        ensure(
            self.now() >= row["issuedAt"].as_i64().unwrap_or(i64::MAX)
                && self.now() < row["expiresAt"].as_i64().unwrap_or(0)
                && self.now() >= row["lastSeenAt"].as_i64().unwrap_or(0),
            "SESSION_EXPIRED",
            "Tu sesión terminó. Inicia sesión para continuar.",
        )?;
        ensure(
            row["locked"] != true,
            "SESSION_LOCKED",
            "La pantalla está bloqueada. Ingresa tu contraseña para continuar.",
        )?;
        let last_seen = row["lastSeenAt"].as_i64().unwrap_or(0);
        let actor: Actor = serde_json::from_value(row)?;
        ensure(
            (0..=2).contains(&actor.role),
            "FORBIDDEN",
            "El rol de esta cuenta no es compatible. Solicita ayuda al administrador.",
        )?;
        ensure(
            actor.can(permission),
            "FORBIDDEN",
            "Tu cuenta no tiene permiso para realizar esta operación. Solicita ayuda al administrador.",
        )?;
        if self.now() > last_seen {
            self.db()?.execute(
                "UPDATE UserSession SET LastSeenAt=?1 WHERE TokenHash=?2",
                rusqlite::params![self.now(), auth::digest(token)],
            )?;
        }
        Ok(actor)
    }
    /// Correlation only: this value cannot authenticate a user. It prevents a
    /// queued command from acting on a different login or active database.
    pub fn ui_context(&self) -> Option<String> {
        self.session_token
            .as_ref()
            .map(|token| auth::digest(&format!("ui-context:{}:{token}", self.db_path.display())))
    }
    pub fn check_ui_context(&self, expected: Option<&str>) -> Result<()> {
        ensure(
            self.ui_context().as_deref() == expected,
            "SESSION_CHANGED",
            "La sesión cambió. Vuelve a abrir esta operación desde tu cuenta.",
        )
    }
    pub fn bootstrap(&self) -> Result<Value> {
        let profile: Value = serde_json::from_str(include_str!(
            "../../../assets/branding/customer-profile.json"
        ))?;
        if let Some(error) = &self.startup_error {
            return Ok(
                json!({"storageError":error,"profile":profile,"version":env!("CARGO_PKG_VERSION"),"demo":self.demo,"uiContext":self.ui_context()}),
            );
        }
        let setup = db::scalar(self.db()?, "SELECT COUNT(*) FROM User", [])? == 0;
        let mut settings = db::all(self.db()?, "SELECT s.*,COALESCE(p.Subtitle,'') AS Subtitle,COALESCE(p.AutoPrint,0) AS AutoPrint FROM StoreSettings s LEFT JOIN StorePresentation p ON p.StoreId=s.Id LIMIT 1", [])?
            .into_iter()
            .next();
        let actor = self.actor("sell").ok();
        if actor.as_ref().is_none_or(|a| a.role != 0) {
            if let Some(s) = settings.as_mut().and_then(Value::as_object_mut) {
                s.remove("backupDirectory");
                s.remove("logoPath");
            }
        }
        let expires = if actor.is_some() {
            db::scalar(
                self.db()?,
                "SELECT ExpiresAt FROM UserSession WHERE TokenHash=?1",
                [auth::digest(self.session_token.as_deref().unwrap_or(""))],
            )
            .ok()
        } else {
            None
        };
        let preferences = if let Some(actor) = &actor {
            db::one(
                self.db()?,
                "SELECT * FROM UserPreferences WHERE UserId=?1",
                [actor.id],
            )
            .ok()
        } else {
            None
        };
        let locked = if let Some(token) = &self.session_token {
            db::scalar(
                self.db()?,
                "SELECT COALESCE(MAX(Locked),0) FROM UserSession WHERE TokenHash=?1",
                [auth::digest(token)],
            )? > 0
        } else {
            false
        };
        let database_id = db::one(
            self.db()?,
            "SELECT Value FROM AppMeta WHERE Key='databaseId'",
            [],
        )?;
        let appearance = if actor.is_some() {
            self.appearance().ok()
        } else {
            None
        };
        Ok(
            json!({"needsSetup":setup,"settings":settings,"actor":actor,"preferences":preferences,"appearance":appearance,"profile":profile,"version":env!("CARGO_PKG_VERSION"),"demo":self.demo,"locked":locked,"storageError":null,"databaseId":database_id["value"],"sessionExpiresAt":expires,"uiContext":self.ui_context()}),
        )
    }
    pub fn log_error(&self, module: &str, operation: &str, error: &AppError) {
        use std::io::Write;
        let dir = self.root.join("logs");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{}.jsonl", &self.now_string()[..10])))
        {
            let _ = writeln!(
                file,
                "{}",
                json!({"time":self.now_string(),"code":error.code,"module":module,"operation":operation,"incidentId":error.incident_id,"message":error.message,"technical":error.technical})
            );
        }
    }
}
