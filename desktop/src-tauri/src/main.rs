#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use donovan_core::commands::AppState;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a == "--demo");
    let root = if demo {
        std::env::temp_dir()
            .join("DonovanPOS2-Demo")
            .join(uuid::Uuid::new_v4().to_string())
    } else if let Some(index) = args.iter().position(|a| a == "--data-dir") {
        args.get(index + 1)
            .map(PathBuf::from)
            .unwrap_or_else(default_data)
    } else {
        default_data()
    };
    let state = Arc::new(AppState {
        engine: Mutex::new(None),
        root,
        demo,
    });
    let result = tauri::Builder::default()
        .manage(state.clone())
        .setup(move |app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut next_attempt = std::time::Instant::now();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    if std::time::Instant::now() < next_attempt {
                        continue;
                    }
                    if let Ok(mut guard) = state.engine.try_lock() {
                        if let Some(Ok(engine)) = guard.as_mut() {
                            if engine.startup_error.is_none() {
                                match engine.automatic_backup_due() {
                                    Ok(Some(_)) => {
                                        let _ = handle.emit("backup-status-changed", ());
                                    }
                                    Ok(None) => {}
                                    Err(error) => {
                                        engine.log_error("storage", "automatic-backup", &error);
                                        next_attempt = std::time::Instant::now()
                                            + std::time::Duration::from_secs(900);
                                        let _ = handle.emit("backup-status-changed", ());
                                    }
                                }
                            }
                        }
                    }
                }
            });
            if let Some(window) = app.get_webview_window("main") {
                if let Some(monitor) = window.current_monitor()? {
                    let available = monitor
                        .work_area()
                        .size
                        .to_logical::<f64>(monitor.scale_factor());
                    window.set_size(tauri::LogicalSize::new(
                        1366f64.min(available.width - 16.),
                        850f64.min((available.height - 64.).max(400.)),
                    ))?;
                    window.center()?;
                }
                window.set_title(if demo {
                    "Donovan POS Pro 2.0 · DEMOSTRACIÓN"
                } else {
                    "Donovan POS Pro"
                })?;
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![donovan_core::commands::execute])
        .run(tauri::generate_context!());
    if result.is_err() {
        rfd::MessageDialog::new().set_title("Donovan POS Pro").set_description("No pudimos abrir la ventana de la aplicación. Reinicia el equipo o vuelve a ejecutar el instalador para reparar los componentes. Tus datos se conservan por separado.").set_level(rfd::MessageLevel::Error).show();
    }
}
fn default_data() -> PathBuf {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("AppData/Local")))
        .unwrap_or_else(|| PathBuf::from("C:/ProgramData"));
    local.join("DonovanSystems/DonovanPOS/data-v2")
}
