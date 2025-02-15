mod minecraft;
mod settings;
mod java;

use lazy_static::lazy_static;
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn run_game(java: String, launcher_dir: String, username: String) -> Result<(), String> {
    let memory = settings::JavaMemory { min: "1024".to_string(), max: "4096".to_string() };
    minecraft::run_game(&launcher_dir, &java, &username, "1.21.1", "1.21.1_ver", &memory).await;
    Ok(())
}

#[tauri::command]
async fn run_pack(pack_id: String) -> Result<(), String> {
    minecraft::run_pack(&pack_id).await;
    Ok(())
}

#[tauri::command]
async fn create_pack(pack_id: String, version: String, version_type: String) -> Result<(), String> {
    let settings = settings::load_settings();

    if version_type == "vanilla" {
        minecraft::create_or_fix_vanilla(&settings.packs_dir, &pack_id, &version).await;
    }
    Ok(())
}

#[tauri::command]
fn get_java_list() -> Vec<String> {
    java::get_java_list()
}

#[tauri::command]
fn get_java_version(java_path: String) -> Option<String> {
    java::get_java_version(java_path)
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

#[tauri::command]
fn load_settings() -> settings::Settings {
    settings::load_settings()
}

#[tauri::command]
fn save_settings(settings: settings::Settings) {
    settings::save_settings(&settings);
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
        .invoke_handler(tauri::generate_handler![greet, run_game, get_java_list, get_java_version, get_packs, save_settings, load_settings, run_pack, create_pack])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
