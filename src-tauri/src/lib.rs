mod minecraft;

use lazy_static::lazy_static;
use std::sync::Mutex;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn run_game() {
    minecraft::run_game().await;
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
        .setup(|app| {
            set_app_handle(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, run_game])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
