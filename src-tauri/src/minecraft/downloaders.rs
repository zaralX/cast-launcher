use std::fs::File;
use std::io::Cursor;
use crate::minecraft::{MAX_CONCURRENT_DOWNLOADS};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use reqwest::{get, Client};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use zip::read::ZipArchive;
use tokio::io::AsyncReadExt;

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
        let mut native = false;

        if info.is_null() {
            log::warn!("NOT FOUND ARTIFACT TRYING USE NATIVE");
            info = serde_json::from_str(&data["downloads"]["classifiers"]["natives-windows"].to_string()).unwrap();
            native = true;
            if info.is_null() {
                log::warn!("[NOT FOUND ARTIFACT ANYWHERE] SKIPPED LIB");
                log::warn!("{}", data);
                continue;
            }
        }
        let lib_url = info["url"].as_str().unwrap().to_string();
        let lib_path = info["path"].as_str().unwrap().to_string().split("/").last().unwrap().to_string();
        let full_path = Path::new(dir).join(&lib_path);
        let normalized_path = full_path.clone();

        if normalized_path.exists() {
            lib_paths.push(normalized_path.to_str().unwrap().to_string());
            continue;
        }

        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let path_clone = normalized_path.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            println!("Скачивание библиотеки: {}...", path_clone.display());
            match client.get(&lib_url).send().await {
                Ok(response) => {
                    if let Ok(asset_data) = response.bytes().await {
                        fs::create_dir_all(path_clone.parent().unwrap()).await.unwrap();
                        fs::write(&path_clone, &asset_data).await.unwrap();
                        println!("Библиотека {} загружена!", path_clone.display());

                        if native {
                            println!("Распаковка natives: {}", path_clone.display());
                            if let Err(e) = extract_zip(&path_clone, path_clone.parent().unwrap().parent().unwrap().join("natives").as_path()).await {
                                println!("Ошибка при распаковке {}: {}", path_clone.display(), e);
                            }
                        }
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

async fn extract_zip(zip_path: &Path, extract_to: &Path) -> std::io::Result<()> {
    let zip_path = zip_path.to_owned();
    let extract_to = extract_to.to_owned();

    tokio::task::spawn_blocking(move || {
        let file = File::open(&zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = extract_to.join(file.name());

            if file.is_dir() {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok::<(), std::io::Error>(())
    })
        .await??;

    Ok(())
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
                let normalized_str = os_str.to_string_lossy().replace('/', &std::path::MAIN_SEPARATOR.to_string());
                normalized.push(normalized_str);
            }
            _ => normalized.push(component),
        }
    }

    normalized
}