use crate::{auth,db,error::{AppError,Result,ensure},models::Actor,secure};
use chrono::{DateTime,Utc};
use fs2::FileExt;
use rusqlite::Connection;
use serde_json::{Value,json};
use std::{fs::{File,OpenOptions},path::{Path,PathBuf},sync::Arc};

pub trait Clock:Send+Sync {fn timestamp(&self)->i64;}
pub struct SystemClock;
impl Clock for SystemClock{fn timestamp(&self)->i64{Utc::now().timestamp()}}
pub fn stamp(time:i64)->String{DateTime::from_timestamp(time,0).unwrap_or_default().with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()}
pub struct Engine {
 pub root:PathBuf,pub db_path:PathBuf,pub(crate) connection:Option<Connection>,
 pub(crate) session_token:Option<String>,pub(crate) clock:Arc<dyn Clock>,
 pub demo:bool,pub(crate) demo_password:Option<String>,pub(crate) _lock:File,
 pub last_integrity:Option<String>,pub startup_error:Option<AppError>,
}
impl Engine {
 pub fn open(root:impl AsRef<Path>)->Result<Self>{Self::with_clock(root,Arc::new(SystemClock))}
 pub fn with_clock(root:impl AsRef<Path>,clock:Arc<dyn Clock>)->Result<Self>{
    std::fs::create_dir_all(root.as_ref())?;
    let root=root.as_ref().canonicalize()?;
    let lock=OpenOptions::new().read(true).write(true).create(true).truncate(false).open(root.join(".instance.lock"))?;
    lock.try_lock_exclusive().map_err(|_|AppError::rule("ALREADY_OPEN","Este negocio ya está abierto en otra ventana de Donovan POS Pro."))?;
    let mut db_path=root.join("donovan.db");
    if let Ok(raw)=std::fs::read(root.join("active.json")){
      let config:Value=serde_json::from_slice(&raw)?;
      let relative=config["database"].as_str().ok_or_else(||AppError::rule("INVALID_DATA","Revisa la ubicación configurada de la base."))?;
      ensure(!Path::new(relative).is_absolute() && !Path::new(relative).components().any(|x|matches!(x,std::path::Component::ParentDir|std::path::Component::Prefix(_))),"INVALID_DATA","La ubicación configurada no es válida.")?;
      db_path=root.join(relative);
    }
    let mut engine=Self{root,db_path,connection:None,session_token:None,clock,demo:false,demo_password:None,_lock:lock,last_integrity:None,startup_error:None};
    match engine.open_database(){Ok(())=>{},Err(error)=>engine.startup_error=Some(error)};
    if engine.connection.is_some(){engine.session_token=secure::load(&engine.session_path()).ok().flatten();if engine.actor("sell").is_err(){engine.session_token=None;}}
    Ok(engine)
 }
 pub(crate) fn open_database(&mut self)->Result<()> {
    let existed=self.db_path.exists();
    if let Some(parent)=self.db_path.parent(){std::fs::create_dir_all(parent)?;}
    // Validate a pre-existing file read-only before enabling WAL or migrating anything.
    if existed {let original=db::connect(&self.db_path,true)?;db::compatible(&original)?;
      if db::schema(&original)?<db::SCHEMA_VERSION {
        let safety=self.root.join(format!("before-migration-{}.sqlite",uuid::Uuid::new_v4()));
        let mut target=Connection::open(safety)?;
        rusqlite::backup::Backup::new(&original,&mut target)?.run_to_completion(256,std::time::Duration::from_millis(10),None)?;
      }
    }
    let mut connection=db::connect(&self.db_path,false)?;
    db::migrate(&mut connection,&self.now_string())?;
    self.connection=Some(connection);self.startup_error=None;self.last_integrity=Some(self.now_string());Ok(())
 }
 pub fn db(&self)->Result<&Connection>{self.connection.as_ref().ok_or_else(||self.startup_error.clone().unwrap_or_else(||AppError::rule("STORAGE_UNAVAILABLE","No podemos acceder a la información del negocio.")))}
 pub(crate) fn db_mut(&mut self)->Result<&mut Connection>{self.connection.as_mut().ok_or_else(||AppError::rule("STORAGE_UNAVAILABLE","No podemos acceder a la información del negocio."))}
 pub fn now(&self)->i64{self.clock.timestamp()}
 pub fn now_string(&self)->String{stamp(self.now())}
 pub(crate) fn session_path(&self)->PathBuf{self.root.join("session.dpapi")}
 pub fn actor(&self,permission:&str)->Result<Actor>{
   let token=self.session_token.as_ref().ok_or_else(||AppError::rule("SESSION_EXPIRED","Tu sesión terminó. Inicia sesión para continuar."))?;
   let row=db::one(self.db()?,"SELECT u.Id,u.Username,u.Name,u.Role,s.IssuedAt,s.ExpiresAt,s.LastSeenAt,s.Locked FROM UserSession s JOIN User u ON u.Id=s.UserId WHERE s.TokenHash=?1 AND u.Active=1 AND u.AuthVersion=s.AuthVersion",[auth::digest(token)]).map_err(|_|AppError::rule("SESSION_EXPIRED","Tu sesión terminó. Inicia sesión para continuar."))?;
   ensure(self.now()>=row["issuedAt"].as_i64().unwrap_or(i64::MAX) && self.now()<row["expiresAt"].as_i64().unwrap_or(0) && self.now()>=row["lastSeenAt"].as_i64().unwrap_or(0),"SESSION_EXPIRED","Tu sesión terminó. Inicia sesión para continuar.")?;
   ensure(row["locked"]!=true,"SESSION_LOCKED","La pantalla está bloqueada. Ingresa tu contraseña para continuar.")?;
   let actor:Actor=serde_json::from_value(row)?;
   ensure(actor.can(permission),"FORBIDDEN","Tu cuenta no tiene permiso para realizar esta operación. Solicita ayuda al administrador.")?;
   self.db()?.execute("UPDATE UserSession SET LastSeenAt=?1 WHERE TokenHash=?2",rusqlite::params![self.now(),auth::digest(token)])?;
   Ok(actor)
 }
 pub fn bootstrap(&self)->Result<Value>{
   let profile:Value=serde_json::from_str(include_str!("../../../assets/branding/customer-profile.json"))?;
   if let Some(error)=&self.startup_error{return Ok(json!({"storageError":error,"profile":profile,"version":"2.0.0","demo":self.demo}));}
   let setup=db::scalar(self.db()?,"SELECT COUNT(*) FROM User",[])?==0;
   let settings=db::all(self.db()?,"SELECT * FROM StoreSettings LIMIT 1",[])?.into_iter().next();
   let actor=self.actor("sell").ok();
   let preferences=if let Some(actor)=&actor {db::one(self.db()?,"SELECT * FROM UserPreferences WHERE UserId=?1",[actor.id]).ok()}else{None};
   let locked=if let Some(token)=&self.session_token{db::scalar(self.db()?,"SELECT COALESCE(MAX(Locked),0) FROM UserSession WHERE TokenHash=?1",[auth::digest(token)])?>0}else{false};
   Ok(json!({"needsSetup":setup,"settings":settings,"actor":actor,"preferences":preferences,"profile":profile,"version":"2.0.0","demo":self.demo,"locked":locked,"storageError":null}))
 }
 pub fn log_error(&self,module:&str,operation:&str,error:&AppError){
   use std::io::Write;
   let dir=self.root.join("logs");let _=std::fs::create_dir_all(&dir);
   if let Ok(mut file)=OpenOptions::new().create(true).append(true).open(dir.join(format!("{}.jsonl",&self.now_string()[..10]))){
     let _=writeln!(file,"{}",json!({"time":self.now_string(),"code":error.code,"module":module,"operation":operation,"incidentId":error.incident_id,"technical":error.technical}));
   }
 }
}
