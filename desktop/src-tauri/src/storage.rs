use crate::{
    Engine, db,
    error::{AppError, Result, ensure},
};
use rusqlite::{
    Connection,
    backup::{Backup, StepResult},
    params,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const MAX_DATABASE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub format_version: u32,
    pub application_version: String,
    pub schema_version: i64,
    pub database_id: String,
    pub created_at: String,
    pub store_name: String,
    pub database_bytes: u64,
    pub sha256: String,
}
fn archive_error(error: impl std::fmt::Display) -> AppError {
    AppError::internal(
        "INVALID_BACKUP",
        "El archivo no es un respaldo válido o está incompleto. Selecciona otra copia.",
        error,
    )
}
pub fn display_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else {
        s.strip_prefix("\\\\?\\").unwrap_or(&s).to_owned()
    }
}
pub fn hash_file(path: &Path) -> Result<String> {
    let mut source = BufReader::new(File::open(path)?);
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn require_space(directory: &Path, bytes: u64) -> Result<()> {
    ensure(
        fs2::available_space(directory)? > bytes.saturating_add(64 * 1024 * 1024),
        "NO_SPACE",
        &format!(
            "Se requieren al menos {} MB libres para completar la operación con seguridad. Libera espacio e intenta nuevamente.",
            (bytes + 64 * 1024 * 1024).div_ceil(1024 * 1024)
        ),
    )
}
pub fn managed_directory(root: &Path, group: &str) -> Result<PathBuf> {
    ensure(
        [
            "work",
            "staging",
            "businesses",
            "recovery-preserved",
            "documents",
        ]
        .contains(&group),
        "INVALID_PATH",
        "La ubicación interna no es válida.",
    )?;
    let parent = root.join(group);
    match fs::create_dir(&parent) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    };
    let parent = parent.canonicalize()?;
    ensure(
        parent.starts_with(root) && parent != root,
        "INVALID_PATH",
        "La carpeta de trabajo debe permanecer dentro de los datos de la aplicación.",
    )?;
    let path = parent.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&path)?;
    Ok(path)
}
pub fn remove_managed_directory(root: &Path, path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let resolved = path.canonicalize()?;
    let parent = resolved
        .parent()
        .ok_or_else(|| AppError::rule("INVALID_PATH", "La carpeta no es válida."))?;
    let group = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
    ensure(
        parent.parent() == Some(root)
            && ["work", "staging"].contains(&group)
            && uuid::Uuid::parse_str(resolved.file_name().and_then(|n| n.to_str()).unwrap_or(""))
                .is_ok(),
        "INVALID_PATH",
        "Solo se pueden limpiar carpetas temporales creadas por esta aplicación.",
    )?;
    fs::remove_dir_all(resolved)?;
    Ok(())
}
pub fn snapshot(source: &Connection, target: &Path) -> Result<()> {
    ensure(
        !target.exists(),
        "FILE_EXISTS",
        "El archivo de destino ya existe. Selecciona otra ubicación.",
    )?;
    let pages = db::scalar(source, "PRAGMA page_count", [])? as u64;
    let size = db::scalar(source, "PRAGMA page_size", [])? as u64;
    ensure(
        pages.saturating_mul(size) <= MAX_DATABASE_BYTES,
        "FILE_TOO_LARGE",
        "Esta base supera el límite de respaldo de 8 GB de esta versión.",
    )?;
    require_space(
        target
            .parent()
            .ok_or_else(|| AppError::rule("INVALID_PATH", "Selecciona una carpeta válida."))?,
        pages.saturating_mul(size),
    )?;
    drop(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?,
    );
    let mut destination = db::connect(target, false)?;
    destination.execute_batch("PRAGMA synchronous=FULL; PRAGMA journal_mode=DELETE;")?;
    {
        let copy = Backup::new(source, &mut destination)?;
        let start = Instant::now();
        let mut retries = 0;
        loop {
            ensure(
                start.elapsed() < Duration::from_secs(180),
                "BACKUP_TIMEOUT",
                "El respaldo tardó demasiado. Cierra otras herramientas que usen la base e intenta nuevamente.",
            )?;
            match copy.step(512)? {
                StepResult::Done => break,
                StepResult::More => retries = 0,
                StepResult::Busy | StepResult::Locked => {
                    retries += 1;
                    ensure(
                        retries <= 8,
                        "DATABASE_BUSY",
                        "La base está ocupada por otro programa. Cierra SQLiteStudio u otras herramientas e intenta nuevamente.",
                    )?;
                    std::thread::sleep(Duration::from_millis(100));
                }
                _ => {
                    return Err(AppError::rule(
                        "BACKUP_FAILED",
                        "No pudimos completar la copia de la base.",
                    ));
                }
            }
        }
    }
    destination.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
    db::compatible(&destination)?;
    drop(destination);
    OpenOptions::new().write(true).open(target)?.sync_all()?;
    Ok(())
}
// A final backup is never replaced, even if another process creates the name during the write.
pub fn move_new(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
        let from: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // Both UTF-16 buffers remain alive for the synchronous Win32 call. No replacement flag is used.
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    {
        fs::hard_link(source, destination)?;
        fs::remove_file(source)?;
    }
    Ok(())
}
pub fn write_active(root: &Path, relative: &Path, replace: bool) -> Result<()> {
    let path = root.join(format!("active-{}.partial", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&serde_json::to_vec_pretty(
            &json!({"version":1,"database":relative.to_string_lossy()}),
        )?)?;
        file.sync_all()?;
        drop(file);
        if replace {
            fs::rename(&path, root.join("active.json"))?;
        } else {
            move_new(&path, &root.join("active.json"))?;
        }
        Ok(())
    })();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    result
}
fn read_manifest(archive: &mut ZipArchive<BufReader<File>>) -> Result<Manifest> {
    ensure(
        archive.len() == 2,
        "INVALID_BACKUP",
        "El respaldo contiene archivos inesperados. No se ha restaurado información.",
    )?;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(archive_error)?;
        ensure(
            ["manifest.json", "business.sqlite"].contains(&entry.name()) && !entry.is_dir(),
            "INVALID_BACKUP",
            "El contenido del respaldo no corresponde a Donovan POS Pro.",
        )?;
    }
    let entry = archive.by_name("manifest.json").map_err(archive_error)?;
    ensure(
        entry.size() <= 16384,
        "INVALID_BACKUP",
        "La descripción del respaldo no es válida.",
    )?;
    let manifest: Manifest = serde_json::from_reader(entry.take(16385)).map_err(archive_error)?;
    ensure(
        manifest.format == "donovan-pos-backup"
            && manifest.format_version == 1
            && manifest.database_bytes > 0
            && manifest.database_bytes <= MAX_DATABASE_BYTES
            && manifest.sha256.len() == 64
            && manifest.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            && uuid::Uuid::parse_str(&manifest.database_id).is_ok()
            && manifest.store_name.chars().count() <= 150,
        "INVALID_BACKUP",
        "El formato del respaldo no es compatible o está incompleto.",
    )?;
    ensure(
        (2..=db::SCHEMA_VERSION).contains(&manifest.schema_version),
        "INCOMPATIBLE_DATABASE",
        "Este respaldo necesita una versión compatible de Donovan POS Pro.",
    )?;
    Ok(manifest)
}
pub fn read_archive(path: &Path, target: Option<&Path>) -> Result<Manifest> {
    ensure(
        fs::metadata(path)?.len() <= MAX_DATABASE_BYTES + 64 * 1024 * 1024,
        "FILE_TOO_LARGE",
        "El archivo supera el tamaño admitido por esta versión.",
    )?;
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?)).map_err(archive_error)?;
    let manifest = read_manifest(&mut archive)?;
    let entry = archive.by_name("business.sqlite").map_err(archive_error)?;
    ensure(
        entry.size() == manifest.database_bytes,
        "INVALID_BACKUP",
        "El tamaño de la base no coincide con el respaldo.",
    )?;
    let mut output = if let Some(target) = target {
        require_space(
            target.parent().ok_or_else(|| {
                AppError::rule("INVALID_PATH", "La carpeta de validación no es válida.")
            })?,
            manifest.database_bytes,
        )?;
        Some(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)?,
        )
    } else {
        None
    };
    let mut source = entry.take(manifest.database_bytes + 1);
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut total = 0;
    loop {
        let count = source.read(&mut buffer).map_err(archive_error)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        ensure(
            total <= manifest.database_bytes,
            "INVALID_BACKUP",
            "El archivo contiene más información de la declarada.",
        )?;
        hash.update(&buffer[..count]);
        if let Some(file) = &mut output {
            file.write_all(&buffer[..count])?;
        }
    }
    ensure(
        total == manifest.database_bytes
            && format!("{:x}", hash.finalize()).eq_ignore_ascii_case(&manifest.sha256),
        "INVALID_BACKUP",
        "La copia está dañada o fue modificada. Su contenido no coincide con la comprobación de integridad.",
    )?;
    if let Some(file) = output {
        file.sync_all()?;
    }
    Ok(manifest)
}
pub fn create_archive(
    source: &Connection,
    root: &Path,
    folder: &Path,
    now: &str,
) -> Result<PathBuf> {
    fs::create_dir_all(folder)?;
    let folder = folder.canonicalize()?;
    let work = managed_directory(root, "work")?;
    let destination = folder.join(format!(
        "DonovanPOS-{}-{}.donovanbackup",
        now.replace(['-', ':', ' '], ""),
        &uuid::Uuid::new_v4().to_string()[..8]
    ));
    let partial = destination.with_extension("partial");
    let mut owns_partial = false;
    let result = (|| {
        let database = work.join("business.sqlite");
        snapshot(source, &database)?;
        let frozen = db::connect(&database, true)?;
        let manifest = Manifest {
            format: "donovan-pos-backup".into(),
            format_version: 1,
            application_version: env!("CARGO_PKG_VERSION").into(),
            schema_version: db::schema(&frozen)?,
            database_id: db::one(
                &frozen,
                "SELECT Value FROM AppMeta WHERE Key='databaseId'",
                [],
            )?["value"]
                .as_str()
                .unwrap_or("")
                .into(),
            created_at: now.into(),
            store_name:
                db::one(&frozen, "SELECT StoreName FROM StoreSettings LIMIT 1", [])?["storeName"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
            database_bytes: fs::metadata(&database)?.len(),
            sha256: hash_file(&database)?,
        };
        drop(frozen);
        require_space(&folder, manifest.database_bytes)?;
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&partial)?;
        owns_partial = true;
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(3));
        writer
            .start_file("manifest.json", options)
            .map_err(archive_error)?;
        writer.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
        writer
            .start_file("business.sqlite", options)
            .map_err(archive_error)?;
        std::io::copy(&mut File::open(&database)?, &mut writer)?;
        let file = writer.finish().map_err(archive_error)?;
        file.sync_all()?;
        drop(file);
        read_archive(&partial, None)?;
        move_new(&partial, &destination)?;
        Ok(destination.clone())
    })();
    if owns_partial && partial.exists() {
        let _ = fs::remove_file(&partial);
    }
    let _ = remove_managed_directory(root, &work);
    result
}
impl Engine {
    pub fn save_backup_settings(
        &mut self,
        folder: &Path,
        enabled: bool,
        hours: i64,
        password: &str,
    ) -> Result<Value> {
        let actor = self.reauthenticate(password)?;
        ensure(
            (1..=168).contains(&hours),
            "VALIDATION",
            "El intervalo debe estar entre 1 y 168 horas.",
        )?;
        ensure(
            folder.is_absolute() && folder.is_dir(),
            "INVALID_PATH",
            "Selecciona una carpeta existente para tus respaldos.",
        )?;
        let folder = folder.canonicalize()?;
        let probe = folder.join(format!("DonovanPOS-{}.write-test", uuid::Uuid::new_v4()));
        {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&probe)?;
            file.sync_all()?;
        }
        fs::remove_file(&probe)?;
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        tx.execute("UPDATE StoreSettings SET BackupDirectory=?1,AutomaticBackups=?2,BackupIntervalHours=?3",params![display_path(&folder),enabled,hours])?;
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Configurar respaldos",
            "Instalación",
            "",
        )?;
        tx.commit()?;
        self.data_overview()
    }
    pub fn diagnostics(&self) -> Result<Value> {
        self.actor("settings")?;
        let folder = self.root.join("logs");
        if !folder.exists() {
            return Ok(json!([]));
        }
        let mut files = fs::read_dir(folder)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "jsonl") && p.is_file())
            .collect::<Vec<_>>();
        files.sort();
        files.reverse();
        let mut entries = vec![];
        for file in files.into_iter().take(7) {
            use std::io::{Seek, SeekFrom};
            let mut file = File::open(file)?;
            let size = file.metadata()?.len();
            let start = size.saturating_sub(256 * 1024);
            file.seek(SeekFrom::Start(start))?;
            let mut bytes = vec![];
            file.take(256 * 1024).read_to_end(&mut bytes)?;
            let text = String::from_utf8_lossy(&bytes);
            let mut lines = text.lines();
            if start > 0 {
                lines.next();
            }
            let mut day=lines.filter_map(|line|serde_json::from_str::<Value>(line).ok()).map(|v|json!({"time":v["time"],"code":v["code"],"module":v["module"],"operation":v["operation"],"incidentId":v["incidentId"],"message":v["message"]})).collect::<Vec<_>>();
            day.reverse();
            entries.extend(day);
            if entries.len() >= 200 {
                break;
            }
        }
        entries.truncate(200);
        Ok(json!(entries))
    }
    pub fn data_overview(&self) -> Result<Value> {
        self.actor("settings")?;
        let db = self.db()?;
        Ok(
            json!({"path":display_path(&self.db_path),"root":display_path(&self.root),"bytes":fs::metadata(&self.db_path).map(|m|m.len()).unwrap_or(0)+fs::metadata(format!("{}-wal",self.db_path.display())).map(|m|m.len()).unwrap_or(0),"freeBytes":fs2::available_space(&self.root)?,"schemaVersion":db::schema(db)?,"lastIntegrity":self.last_integrity,"settings":db::one(db,"SELECT BackupDirectory,AutomaticBackups,BackupIntervalHours FROM StoreSettings LIMIT 1",[])?,"migrations":db::all(db,"SELECT * FROM NativeMigration ORDER BY Version",[])?,"history":db::all(db,"SELECT * FROM BackupRecord ORDER BY Id DESC LIMIT 100",[])?,"counts":db::one(db,"SELECT (SELECT COUNT(*) FROM Product) AS Products,(SELECT COUNT(*) FROM Sale) AS Sales,(SELECT COUNT(*) FROM Customer) AS Customers,(SELECT COUNT(*) FROM User) AS Users",[])?}),
        )
    }
    pub fn verify_database(&mut self) -> Result<Value> {
        self.actor("settings")?;
        if let Err(error) = db::compatible(self.db()?) {
            self.startup_error = Some(error.clone());
            self.connection = None;
            return Err(error);
        }
        self.last_integrity = Some(self.now_string());
        Ok(json!({"valid":true,"checkedAt":self.last_integrity,"schemaVersion":db::SCHEMA_VERSION}))
    }
    pub fn create_backup(&mut self, folder: &Path) -> Result<Value> {
        let actor = self.actor("settings")?;
        self.backup_internal(folder, "Manual", Some(actor.id))
    }
    pub(crate) fn backup_internal(
        &mut self,
        folder: &Path,
        kind: &str,
        user: Option<i64>,
    ) -> Result<Value> {
        let now = self.now_string();
        let result = create_archive(self.db()?, &self.root, folder, &now);
        match result {
            Ok(path) => {
                let bytes = fs::metadata(&path)?.len();
                let sha = hash_file(&path)?;
                self.db()?.execute("INSERT INTO BackupRecord(CreatedAt,FilePath,Kind,Successful,FileBytes,Sha256,Result) VALUES (?1,?2,?3,1,?4,?5,'Base, logotipo y preferencias verificados')",params![now,display_path(&path),kind,bytes as i64,sha])?;
                self.db()?.execute("INSERT INTO AppMeta(Key,Value) VALUES ('lastLocalBackupAt',?1) ON CONFLICT(Key) DO UPDATE SET Value=excluded.Value",[self.now().to_string()])?;
                db::audit(
                    self.db()?,
                    &now,
                    user,
                    "Crear respaldo",
                    path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                    kind,
                )?;
                Ok(json!({"path":display_path(&path),"bytes":bytes,"sha256":sha,"createdAt":now}))
            }
            Err(error) => {
                let _=self.db()?.execute("INSERT INTO BackupRecord(CreatedAt,FilePath,Kind,Successful,FileBytes,Sha256,Result) VALUES (?1,?2,?3,0,0,'',?4)",params![now,display_path(folder),kind,error.message]);
                Err(error)
            }
        }
    }
    pub fn automatic_backup_due(&mut self) -> Result<Option<Value>> {
        let Some(settings) = db::all(self.db()?, "SELECT * FROM StoreSettings LIMIT 1", [])?
            .into_iter()
            .next()
        else {
            return Ok(None);
        };
        if settings["automaticBackups"] != true {
            return Ok(None);
        }
        let hours = settings["backupIntervalHours"]
            .as_i64()
            .filter(|h| (1..=168).contains(h))
            .unwrap_or(24);
        let previous = db::all(
            self.db()?,
            "SELECT Value FROM AppMeta WHERE Key='lastLocalBackupAt'",
            [],
        )?
        .into_iter()
        .next()
        .and_then(|v| v["value"].as_str().and_then(|s| s.parse::<i64>().ok()))
        .unwrap_or(0);
        if self.now() - previous < hours * 3600 {
            return Ok(None);
        }
        let folder = PathBuf::from(settings["backupDirectory"].as_str().unwrap_or(""));
        ensure(
            !folder.as_os_str().is_empty(),
            "INVALID_PATH",
            "Selecciona una carpeta de respaldos en Administración de datos.",
        )?;
        self.backup_internal(&folder, "Automático", None).map(Some)
    }
}
