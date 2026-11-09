use serde::Serialize;
use std::fmt;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub incident_id: String,
    pub retryable: bool,
    #[serde(skip_serializing)]
    pub technical: String,
}
impl AppError {
    pub fn rule(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            incident_id: Uuid::new_v4().to_string()[..8].into(),
            retryable: false,
            technical: String::new(),
        }
    }
    pub fn internal(code: &str, message: &str, detail: impl fmt::Display) -> Self {
        Self {
            technical: detail.to_string(),
            retryable: true,
            ..Self::rule(code, message)
        }
    }
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for AppError {}
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        match &e {
            rusqlite::Error::SqliteFailure(error, _)
                if error.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Self::internal(
                    "DATA_CONFLICT",
                    "No pudimos guardar el cambio. Revisa si la clave, código o referencia ya existe.",
                    e,
                )
            }
            _ => Self::internal(
                "STORAGE_UNAVAILABLE",
                "No podemos acceder a la información del negocio. Intenta nuevamente o revisa Administración de datos.",
                e,
            ),
        }
    }
}
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(
            "FILE_ACCESS",
            "No pudimos leer o guardar el archivo. Revisa la ubicación, los permisos y el espacio disponible.",
            e,
        )
    }
}
impl From<mysql::Error> for AppError {
    fn from(error: mysql::Error) -> Self {
        let code = match &error {
            mysql::Error::MySqlError(e) if [1062, 1451, 1452, 3819, 4025].contains(&e.code) => {
                "DATA_CONFLICT"
            }
            mysql::Error::MySqlError(e) if [1205, 1213].contains(&e.code) => "DATABASE_BUSY",
            _ => "STORAGE_UNAVAILABLE",
        };
        // Driver messages may contain business parameters. Do not serialize or
        // persist them; retain only the server error number for diagnostics.
        let detail = match &error {
            mysql::Error::MySqlError(e) => format!("MySQL error {}", e.code),
            mysql::Error::DriverError(e) => format!("MySQL driver: {e}"),
            mysql::Error::IoError(e) => format!("MySQL IO: {:?}", e.kind()),
            _ => "MySQL connection/protocol error".into(),
        };
        Self::internal(
            code,
            "No pudimos completar la operación en MySQL. Conserva la venta y revisa Administración de datos antes de reintentar.",
            detail,
        )
    }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::internal(
            "INVALID_DATA",
            "La información recibida no tiene el formato esperado.",
            e,
        )
    }
}
pub fn ensure(condition: bool, code: &str, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(AppError::rule(code, message))
    }
}
pub fn text(value: &str, label: &str, max: usize) -> Result<String> {
    let value = value.trim();
    ensure(
        !value.is_empty() && value.chars().count() <= max,
        "VALIDATION",
        &format!("{label}: escribe entre 1 y {max} caracteres."),
    )?;
    Ok(value.to_owned())
}
