use std::io;
use std::path::Path;
use serde_json::Value;

mod minecraft;
mod utils;
mod settings;

const VERSION_MANIFEST_LINK: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

const ASSETS_LINK: &str =
    "https://resources.download.minecraft.net/%A/%B";

const FABRIC_LOADERS_BY_GAME_VERSION_LINK: &str =
    "https://meta.fabricmc.net/v2/versions/loader/%A";

const FABRIC_LOADER_LINK: &str =
    "https://meta.fabricmc.net/v2/versions/loader/%A/%B/profile/json";

const FORGE_INSTALLER_LINK: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/%A-%B/forge-%A-%B-installer.jar";
const FORGE_LIST_LINK: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

#[tauri::command]
async fn create_pack(data: Value) -> Result<(), String> {
    let main_dir = Path::new("./test");
    let mut data = data.clone();
    minecraft::create_pack(main_dir, &mut data).await.expect("Failed to create pack");
    Ok(())
}

#[tauri::command]
async fn install_pack(id: &str) -> Result<(), String> {
    let main_dir = Path::new("./test");
    minecraft::install_pack(main_dir, id).await;
    Ok(())
}

#[tauri::command]
async fn run_pack(id: &str) -> Result<(), String> {
    let main_dir = Path::new("./test");
    minecraft::run_pack(main_dir, id).await;
    Ok(())
}

#[tauri::command]
fn get_packs() -> Result<Vec<Value>, String> {
    let main_dir = Path::new("./test");
    let packs = minecraft::get_packs(main_dir);
    Ok(packs)
}

#[tauri::command]
fn get_settings() -> Result<Value, String> {
    let settings = settings::Settings::new().unwrap();
    settings.save().unwrap();
    Ok(settings.data)
}

#[tauri::command]
fn update_settings(data: Value) -> Result<Value, String> {
    let mut settings = settings::Settings::new().unwrap();
    settings.set_data(&data);
    settings.save().unwrap();
    Ok(settings.data)
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
        .invoke_handler(tauri::generate_handler![greet, create_pack, install_pack, run_pack, get_packs, get_settings, update_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
