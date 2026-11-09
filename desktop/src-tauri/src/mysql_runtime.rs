//! Private local MySQL instance. No system service or existing MySQL installation
//! is modified. Binaries are shipped by the installer; data lives outside it.
use crate::{
    database::Connection,
    error::{AppError, Result, ensure},
    mysql_store, secure,
};
use mysql::{OptsBuilder, prelude::Queryable};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use zeroize::Zeroize;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalConfig {
    pub version: u32,
    pub port: u16,
}
#[derive(Serialize, Deserialize)]
struct Secrets {
    root: String,
    app: String,
    admin: String,
}
impl Drop for Secrets {
    fn drop(&mut self) {
        self.root.zeroize();
        self.app.zeroize();
        self.admin.zeroize();
    }
}
fn secret() -> String {
    format!(
        "D{}{}!",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
pub fn hidden(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
fn private_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(windows)]
    {
        let output =
            hidden(Command::new("whoami").args(["/user", "/fo", "csv", "/nh"])).output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let sid = text
            .trim()
            .split(',')
            .next_back()
            .unwrap_or("")
            .trim_matches('"');
        ensure(
            sid.starts_with("S-1-") && sid[4..].bytes().all(|c| c.is_ascii_digit() || c == b'-'),
            "MYSQL_PERMISSIONS",
            "No pudimos proteger la carpeta del motor local.",
        )?;
        let status = hidden(Command::new("icacls").arg(path).args([
            "/inheritance:r",
            "/grant:r",
            &format!("*{sid}:(OI)(CI)F"),
        ]))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
        ensure(
            status.success(),
            "MYSQL_PERMISSIONS",
            "No pudimos proteger los archivos de MySQL.",
        )?;
    }
    Ok(())
}
pub fn configuration(root: &Path) -> Result<LocalConfig> {
    let config: LocalConfig = serde_json::from_slice(&fs::read(root.join("mysql/instance.json"))?)?;
    ensure(
        config.version == 1 && config.port >= 1024,
        "MYSQL_CONFIG",
        "Configuración MySQL incompatible.",
    )?;
    Ok(config)
}
fn credentials(root: &Path) -> Result<Secrets> {
    let mut data=secure::load(&root.join("mysql/credentials.secret"))?.ok_or_else(||AppError::rule("MYSQL_CREDENTIALS","No se encuentran las credenciales locales. Conserva los datos y restaura un respaldo en una instalación nueva."))?;
    let result = serde_json::from_str(&data);
    data.zeroize();
    Ok(result?)
}
fn connect_with(
    root: &Path,
    user: &str,
    password: &str,
    schema: Option<&str>,
) -> Result<mysql::Conn> {
    let config = configuration(root)?;
    let opts = OptsBuilder::new()
        .ip_or_hostname(Some("127.0.0.1"))
        .tcp_port(config.port)
        .user(Some(user))
        .pass(Some(password))
        .db_name(schema)
        .prefer_socket(false)
        .tcp_connect_timeout(Some(Duration::from_secs(3)))
        .read_timeout(Some(Duration::from_secs(120)))
        .write_timeout(Some(Duration::from_secs(120)))
        .stmt_cache_size(128);
    let opts = opts.server_key_path(Some(root.join("mysql/data/public_key.pem")));
    let mut connection = mysql::Conn::new(opts)?;
    let datadir: Option<String> = connection.query_first("SELECT @@datadir")?;
    let actual = PathBuf::from(datadir.unwrap_or_default()).canonicalize()?;
    ensure(
        actual == root.join("mysql/data").canonicalize()?,
        "MYSQL_INSTANCE",
        "El puerto pertenece a otra instancia. No se ha utilizado ni modificado.",
    )?;
    Ok(connection)
}
pub fn root_connection(root: &Path, schema: Option<&str>) -> Result<mysql::Conn> {
    let secrets = credentials(root)?;
    connect_with(root, "donovan_maintenance", &secrets.root, schema)
}
pub fn connect(root: &Path, schema: &str) -> Result<Connection> {
    validate_schema(schema)?;
    let secrets = credentials(root)?;
    Connection::from_mysql(connect_with(
        root,
        "donovan_app",
        &secrets.app,
        Some(schema),
    )?)
}
pub fn validate_schema(schema: &str) -> Result<()> {
    ensure(
        schema.starts_with("donovan_")
            && schema.len() == 40
            && schema[8..].bytes().all(|c| c.is_ascii_hexdigit()),
        "MYSQL_CONFIG",
        "El esquema no pertenece a una base administrada.",
    )
}
pub fn ensure_running(root: &Path, runtime: &Path) -> Result<()> {
    let home = root.join("mysql");
    let config_path = home.join("instance.json");
    let executable = runtime.join(if cfg!(windows) {
        "bin/mysqld.exe"
    } else {
        "bin/mysqld"
    });
    ensure(
        executable.is_file(),
        "MYSQL_RUNTIME_MISSING",
        "El motor MySQL no está instalado. Ejecuta el instalador completo; tus datos se conservan.",
    )?;
    if config_path.exists() && root_connection(root, None).is_ok() {
        return Ok(());
    }
    if home.exists() {
        ensure(
            home.canonicalize()?.starts_with(root),
            "MYSQL_PATH",
            "La instancia debe permanecer dentro de la carpeta persistente del negocio.",
        )?;
    }
    private_directory(&home)?;
    ensure(
        home.canonicalize()?.starts_with(root),
        "MYSQL_PATH",
        "La instancia debe permanecer dentro de la carpeta persistente del negocio.",
    )?;
    let first = !config_path.exists();
    if first {
        ensure(
            !home.join("data").exists(),
            "MYSQL_UNMANAGED_DATA",
            "Ya hay archivos MySQL sin configuración. Se han conservado; restaura un respaldo o solicita diagnóstico.",
        )?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let config = LocalConfig {
            version: 1,
            port: listener.local_addr()?.port(),
        };
        drop(listener);
        let secrets = Secrets {
            root: secret(),
            app: secret(),
            admin: secret(),
        };
        let mut encoded = serde_json::to_string(&secrets)?;
        secure::save(&home.join("credentials.secret"), &encoded)?;
        encoded.zeroize();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&config_path)?;
        file.write_all(&serde_json::to_vec_pretty(&config)?)?;
        file.sync_all()?;
    }
    let config = configuration(root)?;
    // Refuse to collide with any local service, even if it is another MySQL.
    let probe=TcpListener::bind(("127.0.0.1",config.port)).map_err(|_|AppError::rule("MYSQL_PORT_BUSY","El puerto de MySQL está ocupado. Cierra la otra instancia o reinicia el equipo; no se han modificado sus datos."))?;
    drop(probe);
    let base = runtime.canonicalize()?;
    let data = home.join("data");
    let options = home.join("my.cnf");
    let clean = |path: &Path| -> Result<String> {
        let value = crate::storage::display_path(path).replace('\\', "/");
        ensure(
            !value.contains(['\n', '\r', '"']),
            "MYSQL_PATH",
            "La ruta contiene caracteres no admitidos.",
        )?;
        Ok(value)
    };
    let text = format!(
        "[mysqld]\nbasedir=\"{}\"\ndatadir=\"{}\"\nport={}\nbind-address=127.0.0.1\nmysqlx=0\nlower-case-table-names=1\nlocal-infile=0\nsecure-file-priv=NULL\nskip-name-resolve=1\ncharacter-set-server=utf8mb4\ncollation-server=utf8mb4_0900_bin\ninnodb-buffer-pool-size=128M\ninnodb-flush-log-at-trx-commit=1\nskip-log-bin\nmax-allowed-packet=32M\nmax-connections=20\nlog-error=\"{}\"\npid-file=\"{}\"\n",
        clean(&base)?,
        clean(&data)?,
        config.port,
        clean(&home.join("server.log"))?,
        clean(&home.join("server.pid"))?
    );
    fs::write(&options, text)?;
    if !data.exists() {
        let status = hidden(
            Command::new(&executable)
                .arg(format!("--defaults-file={}", options.display()))
                .arg("--initialize"),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
        ensure(
            status.success(),
            "MYSQL_INITIALIZE",
            "No pudimos crear el almacenamiento MySQL. Consulta el registro local del motor.",
        )?;
        fs::write(home.join("bootstrap.pending"), b"1")?;
    }
    let pending = home.join("bootstrap.pending");
    let init = home.join("bootstrap.sql");
    if pending.exists() {
        let secrets = credentials(root)?;
        // Generated credentials contain only ASCII letters, digits and !.
        // Initialization used a random expired root password, never an empty one.
        let mut sql = format!(
            "ALTER USER 'root'@'localhost' IDENTIFIED WITH caching_sha2_password BY '{}';\nCREATE USER IF NOT EXISTS 'donovan_maintenance'@'127.0.0.1' IDENTIFIED WITH caching_sha2_password BY '{}';\nGRANT ALL PRIVILEGES ON *.* TO 'donovan_maintenance'@'127.0.0.1' WITH GRANT OPTION;\nCREATE USER IF NOT EXISTS 'donovan_app'@'127.0.0.1' IDENTIFIED WITH caching_sha2_password BY '{}';\nCREATE USER IF NOT EXISTS 'donovan_admin'@'127.0.0.1' IDENTIFIED WITH caching_sha2_password BY '{}';\n",
            secrets.root, secrets.root, secrets.app, secrets.admin
        );
        fs::write(&init, sql.as_bytes())?;
        sql.zeroize();
    }
    let mut command = Command::new(&executable);
    command.arg(format!("--defaults-file={}", options.display()));
    #[cfg(windows)]
    {
        command.arg("--no-monitor");
    }
    #[cfg(target_os = "macos")]
    {
        command.arg(format!("--socket={}", home.join("mysql.sock").display()));
    }
    if pending.exists() {
        command.arg(format!("--init-file={}", init.display()));
    }
    let mut child = hidden(&mut command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let start = Instant::now();
    loop {
        let result = root_connection(root, None);
        if result.is_ok() {
            if pending.exists() {
                fs::remove_file(&pending)?;
            }
            if init.exists() {
                fs::remove_file(&init)?;
            }
            return Ok(());
        }
        if child.try_wait()?.is_some() {
            return Err(AppError::rule(
                "MYSQL_START",
                "MySQL no pudo iniciar. Revisa mysql/server.log en Administración de datos.",
            ));
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::internal(
                "MYSQL_START_TIMEOUT",
                "No pudimos conectar con MySQL. Conserva los datos y revisa el registro del motor antes de reintentar.",
                result
                    .err()
                    .map(|e| format!("{} {}", e.code, e.technical))
                    .unwrap_or_default(),
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
pub fn create_schema(
    root: &Path,
    source: Option<&Connection>,
    now: &str,
) -> Result<(String, Connection)> {
    let name = format!("donovan_{}", uuid::Uuid::new_v4().simple());
    let quoted = mysql_store::identifier(&name)?;
    let mut admin = root_connection(root, None)?;
    admin.query_drop(format!(
        "CREATE DATABASE {quoted} CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_bin"
    ))?;
    let mut target = Connection::from_mysql(root_connection(root, Some(&name))?)?;
    mysql_store::initialize(&mut target, now)?;
    if let Some(source) = source {
        db_compatible_source(source)?;
        mysql_store::copy(source, &mut target)?;
        mysql_store::compatible(&target)?;
    }
    admin.query_drop(format!(
        "GRANT SELECT,INSERT,UPDATE,DELETE ON {quoted}.* TO 'donovan_app'@'127.0.0.1'"
    ))?;
    admin.query_drop(format!(
        "GRANT ALL PRIVILEGES ON {quoted}.* TO 'donovan_admin'@'127.0.0.1'"
    ))?;
    drop(target);
    Ok((name.clone(), connect(root, &name)?))
}
fn db_compatible_source(source: &Connection) -> Result<()> {
    crate::db::compatible(source)?;
    Ok(())
}
pub fn workbench_details(root: &Path, schema: &str) -> Result<serde_json::Value> {
    validate_schema(schema)?;
    let secrets = credentials(root)?;
    Ok(
        serde_json::json!({"host":"127.0.0.1","port":configuration(root)?.port,"username":"donovan_admin","schema":schema,"password":secrets.admin,"method":"Standard TCP/IP"}),
    )
}
pub fn shutdown(root: &Path) -> Result<()> {
    let port = configuration(root)?.port;
    root_connection(root, None)?.query_drop("SHUTDOWN")?;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(30) {
        // The TCP listener closes before InnoDB has finished flushing. Waiting
        // for the pid file prevents a fast restart racing the old data-file locks.
        if !root.join("mysql/server.pid").exists() && TcpListener::bind(("127.0.0.1", port)).is_ok()
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(AppError::rule(
        "MYSQL_STOP_TIMEOUT",
        "El motor aún está terminando sus operaciones. Espera antes de actualizar la aplicación.",
    ))
}
pub fn stop_if_idle(root: &Path) -> Result<()> {
    use fs2::FileExt;
    if !root.join("mysql/instance.json").exists() {
        return Ok(());
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(".instance.lock"))?;
    lock.try_lock_exclusive().map_err(|_| {
        AppError::rule(
            "ALREADY_OPEN",
            "Cierra Donovan POS Pro antes de actualizar.",
        )
    })?;
    if TcpListener::bind(("127.0.0.1", configuration(root)?.port)).is_ok() {
        return Ok(());
    }
    shutdown(root)
}
