use std::path::Path;
use serde_json::Value;

mod minecraft;
mod utils;

#[tauri::command]
async fn create_pack(data: Value) -> Result<(), String> {
    let main_dir = Path::new("./test");
    let mut data = data.clone();
    minecraft::create_pack(main_dir, &mut data).await.expect("Failed to create pack");
    Ok(())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, create_pack])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
