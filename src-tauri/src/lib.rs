mod minecraft;

use std::process::{Command, Stdio};
use lazy_static::lazy_static;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use std::fs;
use std::io::Read;

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

#[tauri::command]
fn get_packs(launcher_dir: String) -> Vec<serde_json::Value> {
    let mut directories = Vec::new();

    let entries = match fs::read_dir(&launcher_dir) {
        Ok(entries) => entries,
        Err(_) => return directories, // Если ошибка, просто возвращаем пустой список
    };

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                let folder_name = entry.file_name().to_string_lossy().to_string();
                let cast_pack_path = path.join("cast_pack.json");

                let cast_pack: Option<Value> = match fs::File::open(&cast_pack_path) {
                    Ok(mut file) => {
                        let mut contents = String::new();
                        if file.read_to_string(&mut contents).is_ok() {
                            serde_json::from_str(&contents).ok()
                        } else {
                            None
                        }
                    }
                    Err(_) => None, // Если файла нет или ошибка чтения, оставляем None
                };

                directories.push(serde_json::json!({
                    "folder": folder_name,
                    "cast_pack": cast_pack
                }));
            }
        }
    }

    directories
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
        .invoke_handler(tauri::generate_handler![greet, run_game, get_java_list, get_java_version, get_packs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
