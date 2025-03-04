use crate::minecraft::downloaders::download_file;
use crate::minecraft::{downloaders, send_state};
use reqwest::get;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn init(pack_dir: &str, pack_id: &str, version: &str, version_type: &str, java_path: &str) {
    // Папка пака
    fs::create_dir_all(pack_dir)
        .await
        .expect("failed to create pack dir");

    // cast_pack.json
    let cast_pack_file = Path::new(pack_dir).join("cast_pack.json");
    if cast_pack_file.exists() { return; }
    if fs::metadata(&cast_pack_file).await.is_err() {
        fs::write(
            &cast_pack_file,
            format!(
                r#"{{
"pack_id": "{}",
"name": "{}",
"version": "{}",
"type": "{}",
"cast_pack_version": 1,
"installed": false,
"java_path": "{}"
}}"#,
                pack_id, pack_id, version, version_type, java_path
            ),
        )
            .await
            .expect("failed to create cast_pack.json");
    }
}

pub async fn get_version_data(pack_id: &str, version: &str, manifest: serde_json::Value) -> serde_json::Value {
    send_state(pack_id, "version_info", "Получаем информацию о версии");
    let version_url = manifest["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"].as_str().unwrap() == version)
        .unwrap()["url"]
        .as_str()
        .unwrap();

    let version_json = get(version_url).await.unwrap().text().await.unwrap();
    serde_json::from_str(&version_json).unwrap()
}

pub async fn download_client_jar(pack_id: &str, jar_path: &str, version_data: &serde_json::Value) {
    let jar_url = version_data["downloads"]["client"]["url"].as_str().unwrap();
    println!("Скачиваем Minecraft.jar");
    send_state(pack_id, "downloading_jar", "Скачиваем Minecraft.jar");
    download_file(jar_url, jar_path).await;
}

pub async fn download_assets(pack_id: &str, assets_dir: &str, version_data: &serde_json::Value) {
    send_state(pack_id, "downloading_assets", "Загружаем assets");
    let assets_url = version_data["assetIndex"]["url"].as_str().unwrap();
    downloaders::download_assets(assets_url, &assets_dir).await;
}

pub async fn download_libraries(pack_id: &str, libraries_dir: &str, version_data: &serde_json::Value) -> Vec<String> {
    send_state(pack_id, "downloading_libraries", "Загружаем assets");
    let libraries = version_data["libraries"].as_array().unwrap();
    let libs: Vec<String> = downloaders::download_libraries(libraries, &libraries_dir).await;
    println!("Libraries: {}", libs.join(";"));
    libs
}

pub async fn copy_jar_files(src_dir: PathBuf, dest_dir: PathBuf) -> tokio::io::Result<()> {
    let mut dirs_to_delete = vec![];

    // Рекурсивный обход директорий
    let mut stack = vec![src_dir];
    while let Some(current_dir) = stack.pop() {
        let mut entries = fs::read_dir(&current_dir).await?;
        let mut files = Vec::new();

        // Проход по всем файлам и папкам в текущей директории
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map_or(false, |ext| ext == "jar") {
                files.push(path);
            }
        }

        // Копирование файлов .jar в целевую директорию
        for file in files {
            let dest_file = dest_dir.join(file.file_name().unwrap());
            fs::copy(&file, &dest_file).await?;
        }

        // Если директория пуста после копирования, добавляем её в список на удаление
        if fs::read_dir(&current_dir).await.iter().count() == 0 {
            dirs_to_delete.push(current_dir);
        }
    }

    Ok(())
}