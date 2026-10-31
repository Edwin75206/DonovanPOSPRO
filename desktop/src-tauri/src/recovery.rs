use crate::{
    Engine, auth, db,
    error::{AppError, Result, ensure},
    secure, storage,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroize;

pub struct RecoveryPlan {
    pub id: String,
    pub directory: PathBuf,
    pub database: PathBuf,
    pub sha256: String,
    pub created_at: i64,
    pub owner: Option<i64>,
    pub origin: PathBuf,
    pub preview: Value,
}
#[derive(Default)]
pub struct RecoveryState {
    pub plan: Option<RecoveryPlan>,
    pub failures: u8,
    pub locked_until: i64,
}
impl Engine {
    pub fn recovery_access(&self) -> Result<Option<i64>> {
        if self.connection.is_none() {
            return Ok(None);
        }
        if db::scalar(self.db()?, "SELECT COUNT(*) FROM User", [])? == 0 {
            return Ok(None);
        }
        Ok(Some(self.actor("settings")?.id))
    }
    pub fn prepare_recovery(&mut self, source: &Path) -> Result<Value> {
        let owner = self.recovery_access()?;
        ensure(
            source.is_file(),
            "FILE_NOT_FOUND",
            "No encontramos el archivo seleccionado.",
        )?;
        let source = source.canonicalize()?;
        if self.db_path.exists() {
            ensure(
                source != self.db_path.canonicalize()?,
                "ACTIVE_DATABASE",
                "Esta base ya está en uso. Selecciona un respaldo u otra base compatible.",
            )?;
        }
        let directory = storage::managed_directory(&self.root, "staging")?;
        let database = directory.join("business.sqlite");
        let result = (|| {
            let mut signature = [0u8; 16];
            File::open(&source)?
                .read_exact(&mut signature)
                .map_err(|_| {
                    AppError::rule(
                        "INVALID_BACKUP",
                        "El archivo está incompleto. Selecciona una copia válida.",
                    )
                })?;
            let manifest = if &signature == b"SQLite format 3\0" {
                let original = db::connect(&source, true)?;
                db::compatible(&original)?;
                storage::snapshot(&original, &database)?;
                None
            } else {
                Some(storage::read_archive(&source, Some(&database))?)
            };
            let candidate = db::connect(&database, true)?;
            let version = db::compatible(&candidate)?;
            let settings = db::one(
                &candidate,
                "SELECT StoreName,LogoData IS NOT NULL AS HasLogo FROM StoreSettings LIMIT 1",
                [],
            )?;
            let accounts = db::all(
                &candidate,
                "SELECT Username,Name FROM User WHERE Active=1 AND Role=0 ORDER BY Name LIMIT 100",
                [],
            )?;
            ensure(
                !accounts.is_empty(),
                "INVALID_BACKUP",
                "La copia necesita al menos una cuenta administradora activa.",
            )?;
            let identity = if version >= 2 {
                db::one(
                    &candidate,
                    "SELECT Value FROM AppMeta WHERE Key='databaseId'",
                    [],
                )?["value"]
                    .as_str()
                    .map(str::to_owned)
            } else {
                None
            };
            if let Some(manifest) = &manifest {
                ensure(
                    manifest.schema_version == version
                        && identity.as_deref() == Some(&manifest.database_id)
                        && settings["storeName"] == manifest.store_name,
                    "INVALID_BACKUP",
                    "La descripción del respaldo no coincide con la base que contiene.",
                )?;
            }
            let id = uuid::Uuid::new_v4().to_string();
            let preview = json!({"planId":id,"fileName":source.file_name().and_then(|s|s.to_str()).unwrap_or("Respaldo"),"storeName":settings["storeName"],"schemaVersion":version,"targetVersion":db::SCHEMA_VERSION,"legacy":version==0,"createdAt":manifest.as_ref().map(|m|m.created_at.clone()),"bytes":fs::metadata(&database)?.len(),"hasLogo":settings["hasLogo"]==1,"authorization":if owner.is_some(){"current-admin"}else{"backup-admin"},"adminAccounts":accounts,"counts":db::one(&candidate,"SELECT (SELECT COUNT(*) FROM Product) AS Products,(SELECT COUNT(*) FROM Sale) AS Sales,(SELECT COUNT(*) FROM Customer) AS Customers,(SELECT COUNT(*) FROM User) AS Users,(SELECT COUNT(*) FROM Quote) AS Quotes",[])?});
            Ok((preview, id))
        })();
        match result {
            Ok((preview, id)) => {
                let sha256 = storage::hash_file(&database)?;
                if let Some(previous) = self.recovery.plan.take() {
                    let _ = storage::remove_managed_directory(&self.root, &previous.directory);
                }
                self.recovery.plan = Some(RecoveryPlan {
                    id,
                    directory,
                    database,
                    sha256,
                    created_at: self.now(),
                    owner,
                    origin: self.db_path.clone(),
                    preview: preview.clone(),
                });
                Ok(preview)
            }
            Err(error) => {
                let _ = storage::remove_managed_directory(&self.root, &directory);
                Err(error)
            }
        }
    }
    fn prove_backup_admin(
        &mut self,
        database: &Path,
        username: &str,
        password: &str,
    ) -> Result<()> {
        ensure(
            username.len() <= 200 && password.len() <= 512,
            "INVALID_CREDENTIALS",
            "Revisa la cuenta administradora del respaldo.",
        )?;
        ensure(
            self.now() >= self.recovery.locked_until,
            "INVALID_CREDENTIALS",
            "Espera 5 minutos antes de intentar nuevamente.",
        )?;
        let source = db::connect(database, true)?;
        let selected = db::all(
            &source,
            "SELECT PasswordHash FROM User WHERE Role=0 AND Active=1 AND Username=?1 LIMIT 1",
            [username.trim().to_lowercase()],
        )?
        .into_iter()
        .next();
        let fallback = if selected.is_none() {
            db::all(
                &source,
                "SELECT PasswordHash FROM User WHERE Role=0 AND Active=1 LIMIT 1",
                [],
            )?
            .into_iter()
            .next()
        } else {
            None
        };
        let encoded = selected
            .as_ref()
            .or(fallback.as_ref())
            .and_then(|v| v["passwordHash"].as_str())
            .unwrap_or("");
        let valid = auth::password_matches(password, encoded) && selected.is_some();
        if !valid {
            self.recovery.failures += 1;
            if self.recovery.failures >= 5 {
                self.recovery.failures = 0;
                self.recovery.locked_until = self.now() + 300;
            }
            return Err(AppError::rule(
                "INVALID_CREDENTIALS",
                "Utiliza el usuario y contraseña de una cuenta administradora de este respaldo. Tras varios intentos, espera 5 minutos.",
            ));
        }
        self.recovery.failures = 0;
        Ok(())
    }
    fn preserve_unavailable(&self) -> Result<PathBuf> {
        let directory = storage::managed_directory(&self.root, "recovery-preserved")?;
        let mut note = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("origen.json"))?;
        note.write_all(&serde_json::to_vec_pretty(&json!({"originalDatabase":storage::display_path(&self.db_path),"createdAt":self.now_string(),"message":"Los archivos originales permanecen en su ubicación. La recuperación activa una copia nueva sin sobrescribirlos.","reason":self.startup_error.as_ref().map(|e|e.code.clone())}))?)?;
        note.sync_all()?;
        if let Ok(bytes) = fs::read(self.root.join("active.json")) {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join("configuracion-anterior.json"))?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        Ok(directory)
    }
    fn activate(&mut self, path: PathBuf, connection: Connection) -> Result<Value> {
        let relative = path.strip_prefix(&self.root).map_err(|_| {
            AppError::rule(
                "INVALID_PATH",
                "La base preparada debe permanecer en la carpeta de datos.",
            )
        })?;
        storage::write_active(&self.root, relative, true)?;
        // The validated connection is already open before the atomic pointer change.
        let old = self.connection.replace(connection);
        let token_hash = self.session_token.as_ref().map(|s| auth::digest(s));
        self.db_path = path;
        self.startup_error = None;
        self.last_integrity = Some(self.now_string());
        self.importing = Default::default();
        if let Some(mut token) = self.session_token.take() {
            token.zeroize();
        }
        if let (Some(old), Some(hash)) = (&old, token_hash) {
            if let Err(error) = old.execute("DELETE FROM UserSession WHERE TokenHash=?1", [hash]) {
                self.log_error("data", "revoke-previous-session", &error.into());
            }
        }
        drop(old);
        if let Err(error) = secure::remove(&self.session_path()) {
            self.log_error("data", "clear-previous-session", &error);
        }
        if let Some(plan) = self.recovery.plan.take() {
            let _ = storage::remove_managed_directory(&self.root, &plan.directory);
        }
        self.bootstrap()
    }
    pub fn confirm_recovery(
        &mut self,
        plan_id: &str,
        username: &str,
        password: &str,
    ) -> Result<Value> {
        if self.recovery.plan.is_none() {
            if let Ok(db) = self.db() {
                if db::all(
                    db,
                    "SELECT Value FROM AppMeta WHERE Key='lastRestoreOperation'",
                    [],
                )?
                .first()
                .is_some_and(|v| v["value"] == plan_id)
                {
                    return self.bootstrap();
                }
            }
        }
        let plan = self.recovery.plan.as_ref().ok_or_else(|| {
            AppError::rule(
                "PLAN_EXPIRED",
                "Selecciona y valida nuevamente el respaldo.",
            )
        })?;
        ensure(
            plan.id == plan_id
                && plan.origin == self.db_path
                && self.now() >= plan.created_at
                && self.now() - plan.created_at <= 900,
            "PLAN_EXPIRED",
            "La revisión venció o cambió el negocio activo. Selecciona nuevamente el respaldo.",
        )?;
        let source_path = plan.database.clone();
        let expected = plan.sha256.clone();
        let owner = plan.owner;
        let reference = plan.preview["fileName"]
            .as_str()
            .unwrap_or("Respaldo")
            .to_owned();
        ensure(
            self.recovery_access()? == owner,
            "PLAN_CHANGED",
            "La sesión cambió. Revisa nuevamente el respaldo antes de restaurar.",
        )?;
        if owner.is_some() {
            self.reauthenticate(password)?;
        } else {
            self.prove_backup_admin(&source_path, username, password)?;
        }
        ensure(
            storage::hash_file(&source_path)? == expected,
            "BACKUP_CHANGED",
            "La copia preparada cambió después de la revisión. Selecciona nuevamente el respaldo.",
        )?;
        let source = db::connect(&source_path, true)?;
        db::compatible(&source)?;
        let folder = storage::managed_directory(&self.root, "businesses")?;
        let target = folder.join("donovan.db");
        storage::snapshot(&source, &target)?;
        drop(source);
        let mut candidate = db::connect(&target, false)?;
        db::migrate(&mut candidate, &self.now_string())?;
        {
            let tx = candidate.transaction()?;
            tx.execute("DELETE FROM UserSession", [])?;
            tx.execute("UPDATE User SET FailedAttempts=0,LockedUntil=NULL", [])?;
            tx.execute("UPDATE StoreSettings SET BackupDirectory=?1,PrinterName='',LogoPath=CASE WHEN LogoData IS NOT NULL THEN '' ELSE LogoPath END",[storage::display_path(&self.root.join("backups"))])?;
            tx.execute("INSERT INTO AppMeta(Key,Value) VALUES ('lastLocalBackupAt','0') ON CONFLICT(Key) DO UPDATE SET Value='0'",[])?;
            tx.execute("INSERT INTO AppMeta(Key,Value) VALUES ('lastRestoreOperation',?1) ON CONFLICT(Key) DO UPDATE SET Value=excluded.Value",[plan_id])?;
            db::audit(
                &tx,
                &self.now_string(),
                None,
                "Restaurar respaldo",
                &reference,
                "Integridad verificada; archivos anteriores conservados; sesiones anteriores cerradas",
            )?;
            tx.commit()?;
        }
        db::compatible(&candidate)?;
        if owner.is_some() {
            self.backup_internal(&self.root.join("backups"), "Antes de restaurar", owner)?;
        } else if self.startup_error.is_some() {
            self.preserve_unavailable()?;
        }
        self.activate(target, candidate)
    }
    pub fn create_new_database(&mut self, password: &str) -> Result<Value> {
        if self.connection.is_some()
            && db::scalar(self.db()?, "SELECT COUNT(*) FROM User", [])? == 0
        {
            return self.bootstrap();
        }
        let actor = self.reauthenticate(password)?;
        let folder = storage::managed_directory(&self.root, "businesses")?;
        let path = folder.join("donovan.db");
        let mut candidate = db::connect(&path, false)?;
        db::migrate(&mut candidate, &self.now_string())?;
        db::compatible(&candidate)?;
        self.backup_internal(
            &self.root.join("backups"),
            "Antes de crear base",
            Some(actor.id),
        )?;
        self.activate(path, candidate)
    }
    pub fn cancel_recovery(&mut self) -> Result<Value> {
        self.recovery_access()?;
        if let Some(plan) = self.recovery.plan.take() {
            storage::remove_managed_directory(&self.root, &plan.directory)?;
        }
        Ok(json!({"cancelled":true}))
    }
}
