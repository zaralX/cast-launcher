mod minecraft;

use std::fs;
use std::path::Path;
use std::process::Command;
use reqwest::get;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn run_game() {
    minecraft::run_game().await;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, run_game])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
