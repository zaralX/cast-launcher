mod downloaders;

use std::fmt::Debug;
use crate::{emit_global_event, settings};
use crate::minecraft::downloaders::download_file;
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::{get, Client};
use serde_json::json;
use std::path::Path;
use std::process::Command;
use futures::FutureExt;
use tokio::fs;

const MAX_CONCURRENT_DOWNLOADS: usize = 10;

fn send_state(status: &str, pack_id: &str) {
    emit_global_event(
        "downloading",
        json!({
            "status": status,
            "pack_id": pack_id
        }),
    );
}

pub async fn run_pack(pack_id: &str) {
    let settings = settings::load_settings();
    log::info!("settings loaded");

    // Поиск пака
    if fs::metadata(&settings.packs_dir).await.map(|m| m.is_dir()).unwrap_or(false) {
        log::info!("pack folder found");
        let path = Path::new(&settings.packs_dir);
        let file_path = path.join(pack_id).join("cast_pack.json");

        if fs::metadata(&file_path).await.is_ok() {
            log::info!("cast_pack.json found");
        } else {
            send_state("cast_pack.json not found", pack_id);
            log::error!("cast_pack.json not found");
            return;
        }
    } else {
        send_state("pack folder not found", pack_id);
        log::error!("pack folder not found");
        return;
    }

    let profile = settings.profiles.iter().find(|p| p.selected == true).unwrap();

    run_game(&settings.packs_dir, &settings.java_options.path, &profile.username, &settings.java_options.memory).await
}

pub async fn run_game(launcher_dir: &str, java: &str, username: &str, memory: &settings::JavaMemory) {
    let pack_id = "1.21.1_ver";
    let version = "1.21.1";
    let version_type = "vanilla";
    let pack_dir = &format!("{}/{}", launcher_dir, pack_id);

    send_state("Инициализация", pack_id);

    // Папка пака
    fs::create_dir_all(pack_dir)
        .await
        .expect("failed to create pack dir");

    // cast_pack.json
    let cast_pack_file = Path::new(pack_dir).join("cast_pack.json");
    if fs::metadata(&cast_pack_file).await.is_err() {
        fs::write(
            &cast_pack_file,
            format!(
                r#"{{
                  "pack_id": "{}",
                  "name": "{}",
                  "version": "{}",
                  "type": "{}",
                  "cast_pack_version": 1
                }}"#,
                pack_id, pack_id, version, version_type
            ),
        )
            .await
            .expect("failed to create cast_pack.json");
    }

    // Список версий
    send_state("Получение списка версий", pack_id);
    const VERSION_MANIFEST_LINK: &str =
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

    let response = get(VERSION_MANIFEST_LINK)
        .await
        .expect("Error when get version list")
        .text()
        .await
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Последняя версия
    let latest_version = manifest["latest"]["release"].as_str().unwrap();
    println!("Последняя версия: {}", latest_version);

    // JSON Нужной версии
    send_state("Получаем информацию о версии", pack_id);
    let version_url = manifest["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"].as_str().unwrap() == version)
        .unwrap()["url"]
        .as_str()
        .unwrap();

    let version_json = get(version_url).await.unwrap().text().await.unwrap();
    let version_data: serde_json::Value = serde_json::from_str(&version_json).unwrap();

    // Загрузка .jar клиента
    let jar_url = version_data["downloads"]["client"]["url"].as_str().unwrap();
    let jar_path = &format!("{}/client.jar", pack_dir);
    println!("Скачиваем Minecraft.jar");
    send_state("Скачиваем Minecraft.jar", pack_id);
    download_file(jar_url, jar_path).await;
    send_state("Minecraft.jar установлен", pack_id);

    // Загрузка assets
    send_state("Загружаем assets", pack_id);
    let assets_url = version_data["assetIndex"]["url"].as_str().unwrap();
    let assets_dir = format!("{}/assets", pack_dir);
    downloaders::download_assets(assets_url, &assets_dir).await;

    // Загрузка libraries
    send_state("Загружаем libraries", pack_id);
    let libraries = version_data["libraries"].as_array().unwrap();
    let libraries_dir = format!("{}/libraries", pack_dir);
    let libs: Vec<String> = downloaders::download_libraries(libraries, &libraries_dir).await;
    println!("Libraries: {}", libs.join(";"));

    // Запуск игры
    send_state("Запуск игры", pack_id);
    println!("Запуск Minecraft...");
    let mut command = Command::new(java);
    command.arg(format!("-Xms{}M", memory.min)).arg(format!("-Xmx{}M", memory.max));
    command.arg("-cp").arg(format!(
        "{};{};{};{}",
        jar_path,
        libs.join(";"),
        libraries_dir,
        assets_dir
    ));
    command.arg("net.minecraft.client.main.Main");
    command.arg("--username").arg(username);
    command.arg("--accessToken").arg("nothing");
    command.arg("--version").arg(version);
    command.arg("--gameDir").arg(pack_dir);
    command
        .arg("--assetsDir")
        .arg(format!("{}/assets", pack_dir));
    command.arg("--launchTarget").arg("client");
    command.current_dir(pack_dir);

    let program = command.get_program().to_string_lossy();
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    log::info!("Команда запуска: {}", format!("{} {}", program, args));
    command.spawn().expect("Ошибка при запуске Minecraft");
}
