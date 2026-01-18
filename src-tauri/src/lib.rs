use tauri::Manager;
use crate::config::load_config;
use crate::state::app_state::AppState;

mod state;
mod config;
mod commands;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config = load_config(app.handle())?;

            app.manage(AppState {
                config: std::sync::Mutex::new(config),
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::settings::get_config,
            commands::settings::update_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
