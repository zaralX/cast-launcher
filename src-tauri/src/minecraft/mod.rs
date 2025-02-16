mod downloaders;
mod pack_files;

use crate::{emit_global_event, settings};
use reqwest::{get};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use tokio::fs;

const MAX_CONCURRENT_DOWNLOADS: usize = 10;

fn send_state(pack_id: &str, state: &str, status: &str) {
    emit_global_event(
        "launching",
        json!({
            "status": status,
            "state": state,
            "pack_id": pack_id
        }),
    );
}

pub async fn run_pack(pack_id: &str) {
    let settings = settings::load_settings();
    log::info!("settings loaded");

    let mut cast_pack: Option<Value> = None; // Используем Option<Value>

    // Поиск пака
    if fs::metadata(&settings.packs_dir).await.map(|m| m.is_dir()).unwrap_or(false) {
        log::info!("pack folder found");
        let path = Path::new(&settings.packs_dir);
        let file_path = path.join(pack_id).join("cast_pack.json");

        if fs::metadata(&file_path).await.is_ok() {
            log::info!("cast_pack.json found");
            let pack_content = match fs::read_to_string(&file_path).await {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Ошибка чтения файла: {}", e);
                    return;
                }
            };

            match serde_json::from_str::<Value>(&pack_content) {
                Ok(json) => {
                    log::info!("JSON загружен успешно");
                    cast_pack = Some(json);
                }
                Err(e) => {
                    eprintln!("Ошибка парсинга JSON: {}", e);
                    return;
                }
            }
        } else {
            send_state(pack_id, "error", "cast_pack.json not found");
            log::error!("cast_pack.json not found");
            return;
        }
    } else {
        send_state(pack_id, "error", "pack folder not found");
        log::error!("pack folder not found");
        return;
    }

    let profile = settings.profiles.iter().find(|p| p.selected).unwrap();

    // Проверяем, есть ли JSON и версия
    let version = cast_pack.as_ref()
        .and_then(|json| json["version"].as_str())
        .unwrap_or("1.21.1");

    run_game(pack_id, &settings.packs_dir, &settings.java_options.path, &profile.username, version, &settings.java_options.memory).await
}

pub async fn create_or_fix_vanilla(launcher_dir: &str, pack_id: &str, version: &str, java_path: &str) -> Vec<String> {
    let pack_dir = &format!("{}/{}", launcher_dir, pack_id);
    // Инициализация пака
    send_state(pack_id, "init", "Инициализация");
    pack_files::init(pack_dir, pack_id, version, "vanilla", java_path).await;

    // Список версий
    send_state(pack_id, "versions", "Получение списка версий");
    const VERSION_MANIFEST_LINK: &str =
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

    let response = get(VERSION_MANIFEST_LINK)
        .await
        .expect("Error when get version list")
        .text()
        .await
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Data нужной версии
    let version_data: serde_json::Value = pack_files::get_version_data(pack_id, version, manifest).await;

    // Загрузка .jar клиента
    let jar_path = Path::new(pack_dir).join("client.jar").to_string_lossy().into_owned();
    pack_files::download_client_jar(pack_id, &jar_path, &version_data).await;

    // Загрузка assets
    let assets_dir = Path::new(pack_dir).join("assets").to_string_lossy().into_owned();
    pack_files::download_assets(pack_id, &assets_dir, &version_data).await;

    // Загрузка libraries
    let libraries_dir = Path::new(pack_dir).join("libraries").to_string_lossy().into_owned();
    let libs = pack_files::download_libraries(pack_id, &libraries_dir, &version_data).await;

    let mut args: Vec<String> = Vec::new();
    args.push(String::from("-cp"));
    args.push(format!(
        "{};{}",
        jar_path.replace('/', &std::path::MAIN_SEPARATOR.to_string()),
        (libraries_dir + "\\*").replace('/', &std::path::MAIN_SEPARATOR.to_string())
    ));

    // Помечаем что пак был установлен
    let cast_pack_file = Path::new(pack_dir).join("cast_pack.json");
    let cast_pack_data = fs::read_to_string(&cast_pack_file).await.unwrap();
    let mut cast_pack_json: Value = serde_json::from_str(&cast_pack_data).unwrap();

    if let Some(installed) = cast_pack_json.get_mut("installed") {
        *installed = Value::Bool(true);
    }

    fs::write(&cast_pack_file, serde_json::to_string_pretty(&cast_pack_json).unwrap()).await.expect("FAILED UPDATE PACK INSTALLED STATUS");
    send_state(pack_id, "installed", "Версия установлена");

    args
}

pub async fn run_game(pack_id: &str, launcher_dir: &str, java: &str, username: &str, version: &str, memory: &settings::JavaMemory,) {
    let version_type = "vanilla";
    let pack_dir = Path::new(launcher_dir).join(pack_id);

    let mut args: Vec<String> = Vec::new();

    if version_type == "vanilla" {
        args = create_or_fix_vanilla(launcher_dir, pack_id, version, "null".as_ref()).await;
    }

    let cast_pack_path = pack_dir.join("cast_pack.json");
    let pack_settings: Value = serde_json::from_str(fs::read_to_string(cast_pack_path).await.unwrap().as_ref()).expect("failed to read cast_pack.json");

    let mut java_path = java;
    if pack_settings["java_path"] != "launcher" {
        java_path = pack_settings["java_path"].as_str().unwrap()
    }

    // Последняя версия
    // let latest_version = manifest["latest"]["release"].as_str().unwrap();
    // println!("Последняя версия: {}", latest_version);
    
    let natives = pack_dir.join("natives").to_string_lossy().into_owned();

    // Запуск игры
    send_state(pack_id, "starting", "Запуск игры");
    println!("Запуск Minecraft...");
    let mut command = Command::new(java_path);
    command.arg(format!("-Djava.library.path={}", natives));
    command.arg(format!("-Xms{}M", memory.min)).arg(format!("-Xmx{}M", memory.max));
    command.args(args);
    command.arg("net.minecraft.client.main.Main");
    command.arg("--username").arg(username);
    command.arg("--accessToken").arg("nothing");
    command.arg("--version").arg(version);
    command.arg("--gameDir").arg(&pack_dir);
    command
        .arg("--assetsDir")
        .arg(Path::new(&pack_dir).join("assets").to_string_lossy().into_owned());
    command.arg("--launchTarget").arg("client");
    command.current_dir(&pack_dir);

    let program = command.get_program().to_string_lossy();
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    log::info!("Команда запуска: {}", format!("{} {}", program, args));
    command.spawn().expect("Ошибка при запуске Minecraft");
}
