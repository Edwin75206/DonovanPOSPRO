use crate::{
    db,
    engine::{Engine, stamp},
    error::{AppError, Result, ensure, text},
    models::{Actor, Setup},
    secure,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

pub fn digest(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
pub fn password_hash(password: &str) -> String {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut hash = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, 600_000, &mut hash);
    let result = format!(
        "pbkdf2-sha256$600000${}${}",
        STANDARD.encode(salt),
        STANDARD.encode(hash)
    );
    hash.zeroize();
    result
}
pub fn password_matches(password: &str, encoded: &str) -> bool {
    let parts: Vec<_> = encoded.split('$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2-sha256" {
        return false;
    }
    let Ok(rounds) = parts[1].parse::<u32>() else {
        return false;
    };
    if !(100_000..=2_000_000).contains(&rounds) {
        return false;
    }
    let (Ok(salt), Ok(expected)) = (STANDARD.decode(parts[2]), STANDARD.decode(parts[3])) else {
        return false;
    };
    if salt.len() < 16 || salt.len() > 64 || expected.len() != 32 {
        return false;
    }
    let mut actual = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, rounds, &mut actual);
    let valid = bool::from(actual.as_slice().ct_eq(&expected));
    actual.zeroize();
    valid
}
pub fn validate_password(password: &str) -> Result<()> {
    ensure(
        (10..=128).contains(&password.chars().count()),
        "VALIDATION",
        "La contraseña debe tener entre 10 y 128 caracteres.",
    )
}
pub fn authenticate(db: &Connection, username: &str, password: &str, now: i64) -> Result<Actor> {
    let normalized = username.trim().to_lowercase();
    let users = db::all(db, "SELECT * FROM User WHERE Username=?1", [normalized])?;
    let rejected = || {
        AppError::rule(
            "INVALID_CREDENTIALS",
            "Usuario o contraseña incorrectos. Tras varios intentos, espera 5 minutos.",
        )
    };
    let Some(user) = users.first() else {
        let mut dummy = [0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), &[0u8; 16], 600_000, &mut dummy);
        dummy.zeroize();
        return Err(rejected());
    };
    if user["active"] != true {
        return Err(rejected());
    }
    if let Some(until) = user["lockedUntil"].as_str() {
        if until > stamp(now).as_str() {
            return Err(rejected());
        }
    }
    let id = user["id"].as_i64().unwrap_or(0);
    if !password_matches(password, user["passwordHash"].as_str().unwrap_or("")) {
        let attempts = user["failedAttempts"].as_i64().unwrap_or(0) + 1;
        db.execute(
            "UPDATE User SET FailedAttempts=?1,LockedUntil=?2 WHERE Id=?3",
            params![
                if attempts >= 5 { 0 } else { attempts },
                if attempts >= 5 {
                    Some(stamp(now + 300))
                } else {
                    None
                },
                id
            ],
        )?;
        db::audit(
            db,
            &stamp(now),
            Some(id),
            "Acceso rechazado",
            &id.to_string(),
            "",
        )?;
        return Err(rejected());
    }
    db.execute(
        "UPDATE User SET FailedAttempts=0,LockedUntil=NULL WHERE Id=?1",
        [id],
    )?;
    Ok(serde_json::from_value(user.clone())?)
}
impl Engine {
    pub fn setup(&mut self, mut input: Setup) -> Result<Value> {
        ensure(
            db::scalar(self.db()?, "SELECT COUNT(*) FROM User", [])? == 0,
            "ALREADY_CONFIGURED",
            "La instalación ya está configurada. Tus cuentas no se han modificado.",
        )?;
        input.store_name = text(&input.store_name, "Nombre del comercio", 150)?;
        ensure(
            input.address.chars().count() <= 500
                && input.phone.chars().count() <= 60
                && input.tax_id.chars().count() <= 30,
            "VALIDATION",
            "La dirección admite 500 caracteres, el teléfono 60 y el RFC 30.",
        )?;
        input.admin_username =
            text(&input.admin_username, "Usuario administrador", 50)?.to_lowercase();
        input.seller_username =
            text(&input.seller_username, "Usuario vendedor", 50)?.to_lowercase();
        input.admin_name = text(&input.admin_name, "Nombre del administrador", 100)?;
        input.seller_name = text(&input.seller_name, "Nombre del vendedor", 100)?;
        ensure(
            input.admin_username != input.seller_username,
            "VALIDATION",
            "Las dos cuentas deben tener nombres de usuario distintos.",
        )?;
        ensure(
            [58, 80].contains(&input.ticket_width_mm),
            "VALIDATION",
            "Selecciona papel de 58 u 80 mm.",
        )?;
        validate_password(&input.admin_password)?;
        validate_password(&input.seller_password)?;
        let admin_hash = password_hash(&input.admin_password);
        let seller_hash = password_hash(&input.seller_password);
        input.admin_password.zeroize();
        input.seller_password.zeroize();
        let now = self.now_string();
        let backup_dir = crate::storage::display_path(&self.root.join("backups"));
        let tx = self
            .db_mut()?
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for (username, name, hash, role) in [
            (&input.admin_username, &input.admin_name, admin_hash, 0),
            (&input.seller_username, &input.seller_name, seller_hash, 1),
        ] {
            tx.execute("INSERT INTO User(Username,Name,PasswordHash,Role,Active,FailedAttempts,LockedUntil,AuthVersion) VALUES (?1,?2,?3,?4,1,0,NULL,0)",params![username,name,hash,role])?;
            tx.execute(
                "INSERT INTO UserPreferences(UserId) VALUES (?1)",
                [tx.last_insert_rowid()],
            )?;
        }
        tx.execute("INSERT INTO StoreSettings(StoreName,Address,Phone,Email,TaxId,LogoPath,TicketFooter,FolioPrefix,DefaultTaxBasisPoints,TicketWidthMm,PrinterName,BackupDirectory,AutomaticBackups,BackupIntervalHours,DarkMode,ReducedMotion,LogoData) VALUES (?1,?2,?3,'',?4,'','Gracias por construir con nosotros.','V',1600,?5,'',?6,1,24,0,0,?7)",params![input.store_name,input.address,input.phone,input.tax_id,input.ticket_width_mm,backup_dir,include_bytes!("../../../assets/branding/customer-logo-transparent.png").as_slice()])?;
        db::audit(&tx, &now, None, "Configuración inicial", "Instalación", "")?;
        tx.commit()?;
        self.bootstrap()
    }
    pub fn login(&mut self, username: &str, password: &str) -> Result<Value> {
        ensure(
            username.len() <= 200 && password.len() <= 512,
            "INVALID_CREDENTIALS",
            "Revisa tu usuario y contraseña.",
        )?;
        let actor = authenticate(self.db()?, username, password, self.now())?;
        if let Some(token) = &self.session_token {
            self.db()?.execute(
                "DELETE FROM UserSession WHERE TokenHash=?1",
                [digest(token)],
            )?;
        }
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let token = STANDARD.encode(bytes);
        bytes.zeroize();
        let now = self.now();
        let auth_version = db::scalar(
            self.db()?,
            "SELECT AuthVersion FROM User WHERE Id=?1",
            [actor.id],
        )?;
        self.db()?.execute("INSERT INTO UserSession(TokenHash,UserId,AuthVersion,IssuedAt,ExpiresAt,LastSeenAt,Locked) VALUES (?1,?2,?3,?4,?5,?4,0)",params![digest(&token),actor.id,auth_version,now,now+86400])?;
        if let Err(error) = secure::save(&self.session_path(), &token) {
            self.db()?.execute(
                "DELETE FROM UserSession WHERE TokenHash=?1",
                [digest(&token)],
            )?;
            return Err(error);
        }
        self.session_token = Some(token);
        db::audit(
            self.db()?,
            &self.now_string(),
            Some(actor.id),
            "Inicio de sesión",
            &actor.id.to_string(),
            "",
        )?;
        self.bootstrap()
    }
    pub fn logout(&mut self) -> Result<Value> {
        if let Some(token) = &self.session_token {
            self.db()?.execute(
                "DELETE FROM UserSession WHERE TokenHash=?1",
                [digest(token)],
            )?;
        }
        if let Some(mut token) = self.session_token.take() {
            token.zeroize();
        }
        if let Err(error) = secure::remove(&self.session_path()) {
            self.log_error("security", "Cerrar archivo de sesión", &error);
        }
        self.bootstrap()
    }
    pub fn lock_screen(&mut self) -> Result<Value> {
        self.actor("sell")?;
        if let Some(token) = &self.session_token {
            self.db()?.execute(
                "UPDATE UserSession SET Locked=1 WHERE TokenHash=?1",
                [digest(token)],
            )?;
        }
        self.bootstrap()
    }
    pub fn unlock(&mut self, password: &str) -> Result<Value> {
        let token = self
            .session_token
            .as_ref()
            .ok_or_else(|| AppError::rule("SESSION_EXPIRED", "Inicia sesión para continuar."))?;
        let row = db::one(
            self.db()?,
            "SELECT u.Username,s.IssuedAt,s.ExpiresAt,s.LastSeenAt FROM UserSession s JOIN User u ON s.UserId=u.Id AND s.AuthVersion=u.AuthVersion WHERE s.TokenHash=?1",
            [digest(token)],
        )?;
        ensure(
            self.now() >= row["lastSeenAt"].as_i64().unwrap_or(i64::MAX)
                && self.now() < row["expiresAt"].as_i64().unwrap_or(0),
            "SESSION_EXPIRED",
            "Tu sesión terminó. Inicia sesión para continuar.",
        )?;
        authenticate(
            self.db()?,
            row["username"].as_str().unwrap_or(""),
            password,
            self.now(),
        )?;
        self.db()?.execute(
            "UPDATE UserSession SET Locked=0 WHERE TokenHash=?1",
            [digest(token)],
        )?;
        self.bootstrap()
    }
    pub fn reauthenticate(&self, password: &str) -> Result<Actor> {
        let actor = self.actor("settings")?;
        authenticate(self.db()?, &actor.username, password, self.now())
    }
    pub fn users(&self) -> Result<Value> {
        self.actor("users")?;
        Ok(json!(db::all(
            self.db()?,
            "SELECT Id,Username,Name,Role,Active,FailedAttempts,LockedUntil FROM User ORDER BY Name",
            []
        )?))
    }
    pub fn save_user(
        &mut self,
        id: i64,
        username: &str,
        name: &str,
        role: i64,
        active: bool,
        password: &str,
        admin_password: &str,
    ) -> Result<Value> {
        let actor = self.reauthenticate(admin_password)?;
        let username = text(username, "Usuario", 50)?.to_lowercase();
        let name = text(name, "Nombre", 100)?;
        ensure(
            [0, 1, 2].contains(&role),
            "VALIDATION",
            "Selecciona un rol válido.",
        )?;
        ensure(
            id != actor.id || (active && role == 0),
            "VALIDATION",
            "No puedes desactivar tu cuenta ni quitarte el rol de administrador.",
        )?;
        if id == 0 || !password.is_empty() {
            validate_password(password)?;
        }
        let hash = if password.is_empty() {
            None
        } else {
            Some(password_hash(password))
        };
        let now = self.now_string();
        let tx = self.db_mut()?.transaction()?;
        if id != 0 {
            db::one(&tx, "SELECT Id FROM User WHERE Id=?1", [id])?;
            if !active || role != 0 {
                ensure(
                    db::scalar(
                        &tx,
                        "SELECT COUNT(*) FROM User WHERE Active=1 AND Role=0 AND Id<>?1",
                        [id],
                    )? > 0,
                    "LAST_ADMIN",
                    "Debe quedar al menos un administrador activo.",
                )?;
            }
            tx.execute("UPDATE User SET Username=?1,Name=?2,Role=?3,Active=?4,PasswordHash=COALESCE(?5,PasswordHash),AuthVersion=AuthVersion+1,FailedAttempts=0,LockedUntil=NULL WHERE Id=?6",params![username,name,role,active,hash,id])?;
            tx.execute("DELETE FROM UserSession WHERE UserId=?1", [id])?;
        } else {
            tx.execute("INSERT INTO User(Username,Name,PasswordHash,Role,Active,FailedAttempts,LockedUntil,AuthVersion) VALUES (?1,?2,?3,?4,?5,0,NULL,0)",params![username,name,hash,role,active])?;
            tx.execute(
                "INSERT INTO UserPreferences(UserId) VALUES (?1)",
                [tx.last_insert_rowid()],
            )?;
        }
        db::audit(
            &tx,
            &now,
            Some(actor.id),
            "Guardar usuario",
            &id.to_string(),
            "",
        )?;
        tx.commit()?;
        Ok(json!({"saved":true,"reauthenticate":id==actor.id}))
    }
    pub fn tutorial(&mut self, status: &str, step: i64) -> Result<Value> {
        let actor = self.actor("sell")?;
        let preferences = db::one(
            self.db()?,
            "SELECT * FROM UserPreferences WHERE UserId=?1",
            [actor.id],
        )?;
        self.preferences(
            status,
            step,
            preferences["theme"].as_str().unwrap_or("system"),
            preferences["reducedMotion"] == true,
        )
    }
    pub fn preferences(
        &mut self,
        status: &str,
        step: i64,
        theme: &str,
        reduced: bool,
    ) -> Result<Value> {
        let actor = self.actor("sell")?;
        ensure(
            ["pending", "skipped", "in_progress", "completed"].contains(&status)
                && (0..=30).contains(&step)
                && ["system", "light", "dark"].contains(&theme),
            "VALIDATION",
            "La preferencia seleccionada no es válida.",
        )?;
        self.db()?.execute("INSERT INTO UserPreferences(UserId,TutorialStatus,TutorialStep,Theme,ReducedMotion) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(UserId) DO UPDATE SET TutorialStatus=excluded.TutorialStatus,TutorialStep=excluded.TutorialStep,Theme=excluded.Theme,ReducedMotion=excluded.ReducedMotion",params![actor.id,status,step,theme,reduced])?;
        db::one(
            self.db()?,
            "SELECT * FROM UserPreferences WHERE UserId=?1",
            [actor.id],
        )
    }
}
