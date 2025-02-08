mod downloaders;

use std::path::Path;
use std::process::Command;
use reqwest::{get, Client};
use tokio::fs;
use futures::stream::{FuturesUnordered, StreamExt};

const MAX_CONCURRENT_DOWNLOADS: usize = 10;

pub async fn run_game() {
    let version = "1.21.1";
    let version_type = "vanilla";
    let username = "CL_001";
    let launcher_dir = "D:/RustProjects/cast-launcher/test";
    let java = "C:/Users/Miste/.jdks/graalvm-ce-21.0.2/bin/java.exe";

    // Список версий
    const VERSION_MANIFEST_LINK: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

    let response = get(VERSION_MANIFEST_LINK).await
        .expect("Error when get version list")
        .text().await
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Последняя версия
    let latest_version = manifest["latest"]["release"].as_str().unwrap();
    println!("Последняя версия: {}", latest_version);

    // JSON Нужной версии
    let version_url = manifest["versions"].as_array().unwrap()
        .iter()
        .find(|v| v["id"].as_str().unwrap() == version)
        .unwrap()["url"].as_str().unwrap();

    let version_json = get(version_url).await.unwrap().text().await.unwrap();
    let version_data: serde_json::Value = serde_json::from_str(&version_json).unwrap();

    // Загрузка .jar клиента
    let jar_url = version_data["downloads"]["client"]["url"].as_str().unwrap();
    let jar_path = format!("{}/minecraft_{}.jar", launcher_dir, version);

    if !Path::new(&jar_path).exists() {
        println!("Скачивание Minecraft: {}...", version);
        let jar_data = get(jar_url).await.unwrap().bytes().await.unwrap();
        fs::write(&jar_path, jar_data).await.unwrap();
        println!("Minecraft {} загружен!", version);
    }

    // Загрузка assets
    let assets_url = version_data["assetIndex"]["url"].as_str().unwrap();
    let assets_dir = format!("{}/assets", launcher_dir);
    downloaders::download_assets(assets_url, &assets_dir).await;

    // Загрузка libraries
    let libraries = version_data["libraries"].as_array().unwrap();
    let libraries_dir = format!("{}/libraries", launcher_dir);
    let libs: Vec<String> = downloaders::download_libraries(libraries, &libraries_dir).await;
    println!("Libraries: {}", libs.join(";"));

    // Запуск игры
    println!("Запуск Minecraft...");
    let mut command = Command::new(java);
    command.arg("-Xms1G").arg("-Xmx4G");
    command.arg("-cp").arg(format!("{};{};{};{}", jar_path, libs.join(";"), libraries_dir, assets_dir));
    command.arg("net.minecraft.client.main.Main");
    command.arg("--username").arg(username);
    command.arg("--accessToken").arg("nothing");
    command.arg("--version").arg(version);
    command.arg("--gameDir").arg(launcher_dir);
    command.arg("--assetsDir").arg(format!("{}/assets", launcher_dir));
    command.arg("--launchTarget").arg("client");

    let program = command.get_program().to_string_lossy();
    let args = command.get_args().map(|arg| arg.to_string_lossy()).collect::<Vec<_>>().join(" ");
    println!("Команда запуска: {}", format!("{} {}", program, args));
    println!("Запуск: {:?} {:?}", java, command.get_args().collect::<Vec<_>>());
    command.spawn().expect("Ошибка при запуске Minecraft");
}