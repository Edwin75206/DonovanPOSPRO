#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use donovan_core::{Engine,commands::AppState};
use std::{path::PathBuf,sync::{Arc,Mutex}};
fn main(){
 let args:Vec<String>=std::env::args().collect();let demo=args.iter().any(|a|a=="--demo");
 let root=if demo{std::env::temp_dir().join("DonovanPOS2-Demo").join(uuid::Uuid::new_v4().to_string())}else if let Some(index)=args.iter().position(|a|a=="--data-dir"){args.get(index+1).map(PathBuf::from).unwrap_or_else(default_data)}else{default_data()};
 let mut engine=Engine::open(&root);if demo{engine=engine.and_then(|mut e|{e.seed_demo()?;Ok(e)});}
 tauri::Builder::default().manage(Arc::new(AppState{engine:Mutex::new(engine),root})).invoke_handler(tauri::generate_handler![donovan_core::commands::execute]).run(tauri::generate_context!()).expect("No se pudo iniciar la ventana de Donovan POS Pro");
}
fn default_data()->PathBuf{let local=std::env::var_os("LOCALAPPDATA").map(PathBuf::from).or_else(||std::env::var_os("USERPROFILE").map(|p|PathBuf::from(p).join("AppData/Local"))).unwrap_or_else(||PathBuf::from("C:/ProgramData"));local.join("DonovanSystems/DonovanPOS/data-v2")}
