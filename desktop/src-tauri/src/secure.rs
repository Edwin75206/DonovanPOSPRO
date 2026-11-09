use crate::error::{AppError, Result};
use std::path::Path;
use zeroize::Zeroize;

#[cfg(windows)]
fn protect(bytes: &[u8], decrypt: bool) -> Result<Vec<u8>> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI current-user scope. Only an opaque random session token is persisted, never a password.
    let ok = unsafe {
        if decrypt {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err(AppError::internal(
            "SESSION_STORAGE",
            "No pudimos recuperar la sesión guardada. Inicia sesión nuevamente.",
            std::io::Error::last_os_error(),
        ));
    }
    let data =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
        LocalFree(output.pbData as *mut _);
    }
    Ok(data)
}
#[cfg(not(any(windows, target_os = "macos")))]
fn protect(_bytes: &[u8], _decrypt: bool) -> Result<Vec<u8>> {
    Err(AppError::rule(
        "WINDOWS_REQUIRED",
        "El almacenamiento seguro de sesión requiere Windows.",
    ))
}
#[cfg(not(target_os = "macos"))]
pub fn save(path: &Path, token: &str) -> Result<()> {
    let bytes = protect(token.as_bytes(), false)?;
    let temp = path.with_extension(format!("{}.new", uuid::Uuid::new_v4()));
    std::fs::write(&temp, bytes)?;
    // Replace atomically: a failed save must preserve the last valid secret.
    std::fs::rename(temp, path)?;
    Ok(())
}
#[cfg(not(target_os = "macos"))]
pub fn load(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut decrypted = protect(&std::fs::read(path)?, true)?;
    let result = String::from_utf8(decrypted.clone()).map_err(|_| {
        AppError::rule(
            "SESSION_STORAGE",
            "Inicia sesión nuevamente para continuar.",
        )
    });
    decrypted.zeroize();
    Ok(Some(result?))
}
#[cfg(not(target_os = "macos"))]
pub fn remove(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "com.donovan.pos.local";
#[cfg(target_os = "macos")]
pub fn save(path: &Path, token: &str) -> Result<()> {
    security_framework::passwords::set_generic_password(
        KEYCHAIN_SERVICE,
        &path.to_string_lossy(),
        token.as_bytes(),
    )
    .map_err(|e| {
        AppError::internal(
            "SECURE_STORAGE",
            "No pudimos guardar el acceso en el llavero de macOS.",
            e.code(),
        )
    })?;
    std::fs::write(path, b"Donovan POS: secret stored in macOS Keychain v1")?;
    Ok(())
}
#[cfg(target_os = "macos")]
pub fn load(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut bytes = security_framework::passwords::get_generic_password(
        KEYCHAIN_SERVICE,
        &path.to_string_lossy(),
    )
    .map_err(|e| {
        AppError::internal(
            "SECURE_STORAGE",
            "No pudimos recuperar el acceso del llavero de macOS.",
            e.code(),
        )
    })?;
    let value = String::from_utf8(bytes.clone())
        .map_err(|_| AppError::rule("SECURE_STORAGE", "El acceso guardado no es válido."));
    bytes.zeroize();
    Ok(Some(value?))
}
#[cfg(target_os = "macos")]
pub fn remove(path: &Path) -> Result<()> {
    if let Err(e) = security_framework::passwords::delete_generic_password(
        KEYCHAIN_SERVICE,
        &path.to_string_lossy(),
    ) {
        if e.code() != -25300 {
            return Err(AppError::internal(
                "SECURE_STORAGE",
                "No pudimos retirar el acceso del llavero de macOS.",
                e.code(),
            ));
        }
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
