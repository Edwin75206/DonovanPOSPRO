use crate::error::{AppError,Result};
use std::path::Path;
use zeroize::Zeroize;

#[cfg(windows)]
fn protect(bytes:&[u8], decrypt:bool)->Result<Vec<u8>> {
    use windows_sys::Win32::{Security::Cryptography::{CRYPT_INTEGER_BLOB,CryptProtectData,CryptUnprotectData,CRYPTPROTECT_UI_FORBIDDEN},Foundation::LocalFree};
    let input=CRYPT_INTEGER_BLOB{cbData:bytes.len() as u32,pbData:bytes.as_ptr() as *mut u8};
    let mut output=CRYPT_INTEGER_BLOB{cbData:0,pbData:std::ptr::null_mut()};
    // DPAPI current-user scope. Only an opaque random session token is persisted, never a password.
    let ok=unsafe{if decrypt{CryptUnprotectData(&input,std::ptr::null_mut(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null(),CRYPTPROTECT_UI_FORBIDDEN,&mut output)}else{CryptProtectData(&input,std::ptr::null(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null(),CRYPTPROTECT_UI_FORBIDDEN,&mut output)}};
    if ok==0{return Err(AppError::internal("SESSION_STORAGE","No pudimos recuperar la sesión guardada. Inicia sesión nuevamente.",std::io::Error::last_os_error()));}
    let data=unsafe{std::slice::from_raw_parts(output.pbData,output.cbData as usize).to_vec()};
    unsafe{std::ptr::write_bytes(output.pbData,0,output.cbData as usize);LocalFree(output.pbData as *mut _);}
    Ok(data)
}
#[cfg(not(windows))]
fn protect(_bytes:&[u8],_decrypt:bool)->Result<Vec<u8>> {Err(AppError::rule("WINDOWS_REQUIRED","El almacenamiento seguro de sesión requiere Windows."))}
pub fn save(path:&Path, token:&str)->Result<()> {
    let bytes=protect(token.as_bytes(),false)?;
    let temp=path.with_extension("new");
    std::fs::write(&temp,bytes)?;
    // Windows rename does not replace an existing file. Removal revokes recovery, never grants access.
    if path.exists(){std::fs::remove_file(path)?;}
    std::fs::rename(temp,path)?;Ok(())
}
pub fn load(path:&Path)->Result<Option<String>> {
    if !path.exists(){return Ok(None);}
    let mut decrypted=protect(&std::fs::read(path)?,true)?;
    let result=String::from_utf8(decrypted.clone()).map_err(|_|AppError::rule("SESSION_STORAGE","Inicia sesión nuevamente para continuar."));
    decrypted.zeroize();Ok(Some(result?))
}
pub fn remove(path:&Path)->Result<()> {if path.exists(){std::fs::remove_file(path)?;}Ok(())}
