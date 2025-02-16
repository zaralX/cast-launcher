use crate::minecraft::{MAX_CONCURRENT_DOWNLOADS};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use reqwest::{get, Client};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;

pub async fn download_file(url: &str, dir: &str) {
    if !Path::new(&dir).exists() {
        let jar_data = get(url).await.unwrap().bytes().await.unwrap();
        fs::write(&dir, jar_data).await.unwrap();
    }
}

pub async fn download_libraries(libs: &Vec<Value>, dir: &str) -> Vec<String> {
    let mut lib_paths = Vec::new();
    let client = Client::new();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
    let mut tasks = FuturesUnordered::new();

    for lib in libs {
        let data: Value = serde_json::from_str(&lib.to_string()).unwrap();
        let mut info: Value = serde_json::from_str(&data["downloads"]["artifact"].to_string()).unwrap();

        if info.is_null() {
            log::warn!("NOT FOUND ARTIFACT TRYING USE WINDOWS NATIVE");
            info = serde_json::from_str(&data["downloads"]["classifiers"]["natives-windows"].to_string()).unwrap();
        }
        let lib_url = info["url"].as_str().unwrap().to_string();
        let lib_path = info["path"].as_str().unwrap().to_string();
        let full_path = Path::new(dir).join(&lib_path);
        
        let normalized_path = normalize_path(&full_path);

        let path = normalized_path.clone(); 

        if path.exists() {
            lib_paths.push(path.to_str().unwrap().to_string()); // Добавляем путь в список
            continue;
        }

        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let path_clone = path.clone();  // Clone path to pass into async task

        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap(); // Блокируем слот

            println!("Скачивание библиотеки: {}...", path_clone.display());
            match client.get(&lib_url).send().await {
                Ok(response) => {
                    if let Ok(asset_data) = response.bytes().await {
                        fs::create_dir_all(Path::new(&path_clone).parent().unwrap())
                            .await
                            .unwrap();
                        fs::write(&path_clone, asset_data).await.unwrap();
                        println!("Библиотека {} загружена!", path_clone.display());
                        Some(path_clone)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    println!("Ошибка при загрузке {}: {}", path_clone.display(), e);
                    None
                }
            }
        }));
    }

    while let Some(result) = tasks.next().await {
        if let Ok(Some(path)) = result {
            lib_paths.push(path.to_str().unwrap().to_string());
        }
    }

    lib_paths
}

pub async fn download_assets(assets_url: &str, assets_dir: &str) {
    let client = Client::new(); // Повторное использование HTTP-клиента
    let response = client
        .get(assets_url)
        .send()
        .await
        .expect("Ошибка загрузки ассетов")
        .text()
        .await
        .unwrap();
    let assets_data: Value = serde_json::from_str(&response).unwrap();

    let objects = assets_data["objects"].as_object().unwrap().clone();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));

    let mut tasks = FuturesUnordered::new();

    for (path, info) in objects {
        let hash = info["hash"].as_str().unwrap().to_string();
        let asset_url = format!(
            "https://resources.download.minecraft.net/{}/{}",
            &hash[..2],
            hash
        );
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
                        fs::create_dir_all(Path::new(&asset_path).parent().unwrap())
                            .await
                            .unwrap();
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

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::Normal(os_str) => {
                // Normalize the path by converting it to a string
                let normalized_str = os_str.to_string_lossy().replace('/', &std::path::MAIN_SEPARATOR.to_string());
                normalized.push(normalized_str);
            }
            _ => normalized.push(component),
        }
    }

    normalized
}