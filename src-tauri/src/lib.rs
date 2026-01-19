use std::process::Command;
use crate::config::load_config;
use crate::state::app_state::AppState;
use tauri::Manager;

mod commands;
mod config;
mod state;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn launch_minecraft(
    java_path: String,
    args: Vec<String>,
) -> Result<(), String> {
    Command::new(java_path)
        .args(args)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
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
            launch_minecraft,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
