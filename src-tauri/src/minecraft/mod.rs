use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use reqwest::{get, Client};
use serde_json::Value;
use tokio::sync::Semaphore;
use tokio::fs;
use futures::stream::{FuturesUnordered, StreamExt};

const MAX_CONCURRENT_DOWNLOADS: usize = 10;

pub async fn run_game() {
    let version = "1.21.1";
    let version_type = "vanilla";
    let username = "CL_001";
    let launcher_dir = "D:\\RustProjects\\cast-launcher\\test";
    let java = "C:/Users/Miste/.jdks/graalvm-ce-21.0.2/bin/javaw.exe";

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
    let jar_path = format!("{}\\minecraft_{}.jar", launcher_dir, version);

    if !Path::new(&jar_path).exists() {
        println!("Скачивание Minecraft: {}...", version);
        let jar_data = get(jar_url).await.unwrap().bytes().await.unwrap();
        fs::write(&jar_path, jar_data).await.unwrap();
        println!("Minecraft {} загружен!", version);
    }

    // Загрузка assets
    let assets_url = version_data["assetIndex"]["url"].as_str().unwrap();
    let assets_dir = format!("{}\\assets", launcher_dir);
    download_assets(assets_url, &assets_dir).await;

    // Загрузка libraries
    let libraries = version_data["libraries"].as_array().unwrap();
    let libraries_dir = format!("{}\\libraries", launcher_dir);
    download_libraries(libraries, &libraries_dir).await;

    // Запуск игры
    println!("Запуск Minecraft...");
    let mut command = Command::new(java);
    command.arg("-Xms1G").arg("-Xmx4G");
    command.arg("--username").arg(username);
    command.arg("--gameDir").arg(launcher_dir);
    command.arg("--assetsDir").arg(format!("{}\\assets", launcher_dir));
    command.arg("--version").arg(version);
    command.arg("--cp").arg("\"minecraft.jar;lwjgl.jar;lwjgl_util.jar\"");
    command.arg("net.minecraft.client.main.Main");
    let command_string = format!(
        "java -Xmx4G -Xms1G --username {} --gameDir {} --assetsDir {} --version {}",
        username,
        launcher_dir,
        format!("{}\\assets", launcher_dir),
        version
    );

    println!("Команда запуска: {}", command_string);
    command.spawn().expect("Ошибка при запуске Minecraft");
}

async fn download_libraries(libs: &Vec<Value>, dir: &str) {
    let client = Client::new(); // Повторное использование HTTP-клиента

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));

    let mut tasks = FuturesUnordered::new();

    for lib in libs {
        let data: Value = serde_json::from_str(&lib.to_string()).unwrap();
        let info: Value = serde_json::from_str(&data["downloads"]["artifact"].to_string()).unwrap();

        let hash = info["sha1"].as_str().unwrap().to_string();
        let lib_url = info["url"].as_str().unwrap().to_string();
        let lib_path = info["path"].as_str().unwrap().to_string();
        let path = format!("{}/{}", dir, lib_path);

        if Path::new(&path).exists() {
            continue;
        }

        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);

        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap(); // Блокируем до освобождения слота

            println!("Скачивание библиотеки: {}...", lib_path);
            match client.get(&lib_url).send().await {
                Ok(response) => {
                    if let Ok(asset_data) = response.bytes().await {
                        fs::create_dir_all(Path::new(&path).parent().unwrap()).await.unwrap();
                        fs::write(&path, asset_data).await.unwrap();
                        println!("Библиотека {} загружена!", lib_path);
                    }
                }
                Err(e) => println!("Ошибка при загрузке {}: {}", path, e),
            }
        }));
    }

    while let Some(_) = tasks.next().await {} // Ждём завершения всех задач
}

async fn download_assets(assets_url: &str, assets_dir: &str) {
    let client = Client::new(); // Повторное использование HTTP-клиента
    let response = client.get(assets_url).send().await.expect("Ошибка загрузки ассетов").text().await.unwrap();
    let assets_data: Value = serde_json::from_str(&response).unwrap();

    let objects = assets_data["objects"].as_object().unwrap().clone();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));

    let mut tasks = FuturesUnordered::new();

    for (path, info) in objects {
        let hash = info["hash"].as_str().unwrap().to_string();
        let asset_url = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
        let asset_path = format!("{}/{}", assets_dir, path);

        if Path::new(&asset_path).exists() {
            continue;
        }

        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);

        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap(); // Блокируем до освобождения слота

            println!("Скачивание ассета: {}...", path);
            match client.get(&asset_url).send().await {
                Ok(response) => {
                    if let Ok(asset_data) = response.bytes().await {
                        fs::create_dir_all(Path::new(&asset_path).parent().unwrap()).await.unwrap();
                        fs::write(&asset_path, asset_data).await.unwrap();
                        println!("Ассет {} загружен!", path);
                    }
                }
                Err(e) => println!("Ошибка при загрузке {}: {}", path, e),
            }
        }));
    }

    while let Some(_) = tasks.next().await {} // Ждём завершения всех задач
}