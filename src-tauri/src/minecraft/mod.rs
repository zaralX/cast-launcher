mod downloaders;
mod pack_files;

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
    pack_files::init(pack_dir, pack_id, version, version_type).await;

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
    // let latest_version = manifest["latest"]["release"].as_str().unwrap();
    // println!("Последняя версия: {}", latest_version);

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
