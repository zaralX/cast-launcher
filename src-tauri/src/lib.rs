mod minecraft;

use std::process::{Command, Stdio};
use lazy_static::lazy_static;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn run_game(java: String, launcher_dir: String, username: String) -> Result<(), String> {
    minecraft::run_game(&launcher_dir, &java, &username).await;
    Ok(())
}

#[tauri::command]
fn get_java_list() -> Vec<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("where").arg("java").output()
    } else {
        Command::new("which").arg("-a").arg("java").output()
    };

    match output {
        Ok(out) => {
            let paths = String::from_utf8_lossy(&out.stdout);
            paths.lines().map(|s| s.trim().to_string()).collect()
        }
        Err(_) => vec![],
    }
}

#[tauri::command]
fn get_java_version(java_path: String) -> Option<String> {
    let output = Command::new(&java_path)
        .arg("-version")
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let version_line = stderr.lines().next()?;
    Some(version_line.to_string())
}

lazy_static! {
    static ref APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);
}

pub fn set_app_handle(handle: AppHandle) {
    let mut global_handle = APP_HANDLE.lock().unwrap();
    *global_handle = Some(handle);
}

pub fn emit_global_event(key: &str, value: Value) {
    if let Some(handle) = APP_HANDLE.lock().unwrap().as_ref() {
        handle.emit(key, value).unwrap();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::LogDir {
                file_name: Some("cast_logs".to_string()),
            },
        )).build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            set_app_handle(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, run_game, get_java_list, get_java_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
