use std::fs::create_dir_all;
use std::path::Path;
use crate::minecraft::cast_pack_json::CastPack;
use crate::minecraft::downloaders;
use crate::minecraft::loaders::fabric;
use crate::minecraft::loaders::fabric::{download_fabric_libraries, get_fabric_version_data};
use crate::minecraft::loaders::vanilla::{download_client_jar, download_vanilla_libraries, get_assets_json, get_client_jar_path, get_version_data};
use crate::utils;
use crate::utils::{extract_jar, get_absolute_path, http_get_json};

pub async fn install(main_dir: &Path, cast_pack: &mut CastPack) {
    let project_id = cast_pack.get("modrinth-project-id").unwrap().as_str().unwrap();
    let project_version = cast_pack.get("modrinth-project-version").unwrap().as_str().unwrap();

    let instance_dir = cast_pack.dir();
    let minecraft_dir = instance_dir.join(".minecraft");
    create_dir_all(minecraft_dir.clone()).unwrap();

    let project_data = http_get_json(format!("https://api.modrinth.com/v2/project/{}/version/{}", project_id, project_version).as_str()).await;

    // MODRINTH Download and extract files[]
    let files = project_data["files"].as_array().unwrap();

    let mrpack_files_dir = instance_dir.join("mrpack");
    create_dir_all(&mrpack_files_dir).unwrap();
    for file in files {
        let filename = file["filename"].as_str().unwrap();
        let url = file["url"].as_str().unwrap();
        let filetype = filename.split(".").last().unwrap();
        if filetype.eq("mrpack") {
            let mrpack_file_path = mrpack_files_dir.join("mrpack.mrpack");
            downloaders::download_file(url, mrpack_file_path.as_path()).await;
            extract_jar(mrpack_file_path.clone(), mrpack_files_dir.clone()).unwrap();

            let mr_pack_overrides_dir = mrpack_files_dir.join("overrides");
            utils::copy_dir_recursive(mr_pack_overrides_dir.as_path(), &*minecraft_dir).unwrap();
        } else {
            println!("UNKNOWN FILE TYPE {}", filename.split(".").last().unwrap())
        }
    }

    // Reading index.json from extracted .mrpack
    let modrinth_index_json_path = mrpack_files_dir.join("modrinth.index.json");
    if !modrinth_index_json_path.exists() {
        panic!("Not found modrinth.index.json")
    }
    
    let modrinth_index_json = utils::read_json_file(modrinth_index_json_path);
    
    // Downloading required files by mrpack
    let modrinth_files = modrinth_index_json["files"].as_array().unwrap();
    for file in modrinth_files {
        let file_path = minecraft_dir.join(file["path"].as_str().unwrap());
        for download in file["downloads"].as_array().unwrap() {
            // TODO Там сделано несколько ссылок для скачивания, тип если не прокнет, тут берётся только первая
            downloaders::download_file(download.as_str().unwrap(), file_path.as_path()).await;
            break
        }
    }
    
    // Reading and downloading requirements
    let version = modrinth_index_json["dependencies"]["minecraft"].as_str().unwrap();
    let fabric_loader = modrinth_index_json["dependencies"]["fabric-loader"].as_str();
    if fabric_loader.is_some() {
        cast_pack.set("version", serde_json::json!(version));
        cast_pack.set("fabric-loader", serde_json::json!(fabric_loader));
        cast_pack.save().unwrap();
        fabric::install(main_dir, cast_pack).await;
    } else {
        panic!("Unknown mod loader for modpack")
    }
}

pub async fn generate_args(main_dir: &Path, cast_pack: &mut CastPack) -> Vec<String> {
    let mut arguments: Vec<String> = Vec::new();
    let minecraft_dir = cast_pack.dir().join(".minecraft");
    create_dir_all(&minecraft_dir).unwrap();

    let modrinth_index_json_path = cast_pack.dir().join("mrpack").join("modrinth.index.json");
    let modrinth_index_json = utils::read_json_file(modrinth_index_json_path);
    let fabric_loader = modrinth_index_json["dependencies"]["fabric-loader"].as_str();
    if fabric_loader.is_some() {
        arguments = fabric::generate_args(main_dir, cast_pack).await;
    } else {
        panic!("Unknown mod loader for modpack")
    }
    
    arguments
}