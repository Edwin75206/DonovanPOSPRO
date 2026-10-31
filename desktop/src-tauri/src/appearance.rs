use crate::{
    Engine, db,
    error::{AppError, Result, ensure},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io::Cursor, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Appearance {
    pub mode: String,
    pub preset: String,
    pub primary: Option<String>,
    pub font_scale: i64,
    pub density: String,
    pub animations: String,
    pub sidebar: String,
    pub background: String,
    pub background_color: String,
    pub image_opacity: i64,
    pub image_x: i64,
    pub image_y: i64,
    pub image_fit: String,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            mode: "system".into(),
            preset: "client".into(),
            primary: None,
            font_scale: 100,
            density: "standard".into(),
            animations: "subtle".into(),
            sidebar: "expanded".into(),
            background: "theme".into(),
            background_color: "#f4f3ef".into(),
            image_opacity: 15,
            image_x: 50,
            image_y: 50,
            image_fit: "cover".into(),
        }
    }
}
fn hex(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|x| x.is_ascii_hexdigit())
}
impl Appearance {
    pub fn validate(&self) -> Result<()> {
        ensure(
            ["system", "light", "dark"].contains(&self.mode.as_str())
                && [
                    "client",
                    "executive",
                    "forest",
                    "turquoise",
                    "night",
                    "minimal",
                ]
                .contains(&self.preset.as_str()),
            "VALIDATION",
            "Selecciona un tema y un modo válidos.",
        )?;
        ensure(
            self.primary.as_ref().is_none_or(|p| hex(p)) && hex(&self.background_color),
            "VALIDATION",
            "Utiliza un color de seis dígitos, por ejemplo #2867B3.",
        )?;
        ensure(
            (90..=150).contains(&self.font_scale)
                && ["compact", "standard", "comfortable"].contains(&self.density.as_str()),
            "VALIDATION",
            "El tamaño de letra debe estar entre 90% y 150%.",
        )?;
        ensure(
            ["none", "subtle", "full"].contains(&self.animations.as_str())
                && ["expanded", "compact"].contains(&self.sidebar.as_str()),
            "VALIDATION",
            "Selecciona una animación y navegación válidas.",
        )?;
        ensure(
            [
                "theme", "light", "dark", "solid", "gray", "tinted", "gradient", "image",
            ]
            .contains(&self.background.as_str())
                && ["cover", "contain"].contains(&self.image_fit.as_str()),
            "VALIDATION",
            "Selecciona un fondo válido.",
        )?;
        ensure(
            (0..=40).contains(&self.image_opacity)
                && (0..=100).contains(&self.image_x)
                && (0..=100).contains(&self.image_y),
            "VALIDATION",
            "Revisa la intensidad y posición del fondo.",
        )
    }
}
pub fn validate_image(bytes: &[u8]) -> Result<()> {
    ensure(
        !bytes.is_empty() && bytes.len() <= 5 * 1024 * 1024,
        "INVALID_IMAGE",
        "Selecciona una imagen PNG o JPEG de hasta 5 MB.",
    )?;
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| AppError::rule("INVALID_IMAGE", "No pudimos leer esta imagen."))?;
    ensure(
        matches!(
            reader.format(),
            Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
        ),
        "INVALID_IMAGE",
        "Utiliza una imagen PNG o JPEG.",
    )?;
    let (width, height) = reader.into_dimensions().map_err(|_| {
        AppError::rule("INVALID_IMAGE", "La imagen está dañada o no es compatible.")
    })?;
    ensure(
        width > 0
            && height > 0
            && width <= 6000
            && height <= 6000
            && (width as u64 * height as u64) <= 20_000_000,
        "INVALID_IMAGE",
        "Utiliza una imagen de hasta 6000 píxeles por lado y 20 megapíxeles.",
    )?;
    image::load_from_memory(bytes)
        .map_err(|_| AppError::rule("INVALID_IMAGE", "La imagen está incompleta o dañada."))?;
    Ok(())
}
impl Engine {
    pub fn appearance(&self) -> Result<Value> {
        let actor = self.actor("sell")?;
        let record = db::all(
            self.db()?,
            "SELECT Version,ConfigJson,BackgroundData FROM UserAppearance WHERE UserId=?1",
            [actor.id],
        )?
        .into_iter()
        .next();
        if let Some(row) = record {
            ensure(
                row["version"] == 1,
                "PREFERENCES_INCOMPATIBLE",
                "Tus preferencias necesitan restablecerse desde Apariencia y personalización.",
            )?;
            let config: Appearance =
                serde_json::from_str(row["configJson"].as_str().unwrap_or("{}"))?;
            config.validate()?;
            return Ok(json!({"config":config,"backgroundData":row["backgroundData"]}));
        }
        let mut config = Appearance::default();
        if actor.role == 1 {
            config.font_scale = 110;
            config.density = "comfortable".into();
            config.sidebar = "compact".into();
        }
        if let Ok(old) = db::one(
            self.db()?,
            "SELECT Theme,ReducedMotion FROM UserPreferences WHERE UserId=?1",
            [actor.id],
        ) {
            config.mode = old["theme"].as_str().unwrap_or("system").into();
            if old["reducedMotion"] == true {
                config.animations = "none".into();
            }
        }
        Ok(json!({"config":config,"backgroundData":null}))
    }
    pub fn save_appearance(
        &mut self,
        config: Appearance,
        background_action: &str,
        background_data: Option<&str>,
    ) -> Result<Value> {
        let actor = self.actor("sell")?;
        config.validate()?;
        ensure(
            ["keep", "replace", "remove"].contains(&background_action),
            "VALIDATION",
            "Selecciona cómo guardar el fondo.",
        )?;
        let bytes = if background_action == "replace" {
            let raw = background_data.unwrap_or("");
            ensure(
                raw.len() <= 7 * 1024 * 1024,
                "INVALID_IMAGE",
                "La imagen supera el tamaño permitido.",
            )?;
            let data = STANDARD
                .decode(raw)
                .map_err(|_| AppError::rule("INVALID_IMAGE", "No pudimos leer esta imagen."))?;
            validate_image(&data)?;
            Some(data)
        } else {
            None
        };
        let tx = self.db_mut()?.transaction()?;
        let has_background = bytes.is_some()
            || background_action == "keep"
                && db::scalar(
                    &tx,
                    "SELECT COUNT(*) FROM UserAppearance WHERE UserId=?1 AND BackgroundData IS NOT NULL",
                    [actor.id],
                )? > 0;
        ensure(
            config.background != "image" || has_background,
            "INVALID_IMAGE",
            "Selecciona una imagen antes de aplicar este fondo.",
        )?;
        tx.execute("INSERT INTO UserAppearance(UserId,ConfigJson,BackgroundData) VALUES (?1,?2,?3) ON CONFLICT(UserId) DO UPDATE SET Version=1,ConfigJson=excluded.ConfigJson",params![actor.id,serde_json::to_string(&config)?,bytes])?;
        if background_action != "keep" {
            tx.execute(
                "UPDATE UserAppearance SET BackgroundData=?1 WHERE UserId=?2",
                params![bytes, actor.id],
            )?;
        }
        tx.commit()?;
        self.appearance()
    }
    pub fn image_for_appearance(&self, path: &Path) -> Result<Value> {
        self.actor("sell")?;
        ensure(
            std::fs::metadata(path)?.len() <= 5 * 1024 * 1024,
            "INVALID_IMAGE",
            "Selecciona una imagen de hasta 5 MB.",
        )?;
        let data = std::fs::read(path)?;
        validate_image(&data)?;
        Ok(json!({"data":STANDARD.encode(data)}))
    }
}
