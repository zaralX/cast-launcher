use std::fs;
use std::fs::create_dir_all;
use std::path::Path;
use crate::minecraft::cast_pack_json::CastPack;
use crate::minecraft::downloaders;
use crate::minecraft::loaders::{fabric, forge};
use crate::minecraft::loaders::fabric::{download_fabric_libraries, get_fabric_version_data};
use crate::minecraft::loaders::vanilla::{download_client_jar, download_vanilla_libraries, get_assets_json, get_client_jar_path, get_version_data};
use crate::utils;
use crate::utils::{extract_jar, get_absolute_path, http_get_json};

pub async fn install(main_dir: &Path, cast_pack: &mut CastPack) {
    let project_id = cast_pack.get("zapi-project-id").unwrap().as_str().unwrap();
    let mut project_version = cast_pack.get("zapi-project-version").unwrap().as_str().unwrap().to_string();

    let instance_dir = cast_pack.dir();
    let minecraft_dir = instance_dir.join(".minecraft");
    create_dir_all(minecraft_dir.clone()).unwrap();

    // zAPI getting build(version) data
    let project_data = http_get_json(format!("https://api.zaralx.ru/launcher/project/{}", project_id).as_str()).await;
    if project_version.eq("latest") {
        let project_version_data = project_data["versions"].as_array().unwrap().into_iter().max_by_key(|v| v["build"].as_i64().unwrap()).unwrap();
        project_version = project_version_data["id"].as_i64().unwrap().to_string();
    }
    let version_data = http_get_json(format!("https://api.zaralx.ru/launcher/version/{}", project_version).as_str()).await;

    // zAPI Download and extract files[]
    let files = version_data["data"]["files"].as_array().unwrap();

    for file in files {
        let filename = file["filename"].as_str().unwrap();
        let url = file["url"].as_str().unwrap();
        let path = minecraft_dir.join(file["path"].as_str().unwrap()).join(filename);
        downloaders::download_file(url, path.as_path()).await;
        
        let extract = file["extract"].as_bool();
        if extract.is_some() {
            let extract = extract.unwrap();
            if extract {
                extract_jar(path.clone(), path.parent().unwrap().to_owned().clone()).unwrap();
                fs::remove_file(path).unwrap();
            }
        }
    }
    
    // Reading requirements and installing loader
    let version = version_data["data"]["minecraftVersion"].as_str().unwrap();
    let loader = version_data["data"]["loader"].as_str().unwrap();
    let loader_version = version_data["data"]["loaderVersion"].as_str().unwrap();
    let auto_update = project_data["autoUpdate"].as_bool().unwrap();
    if loader.eq("forge") {
        cast_pack.set("version", serde_json::json!(version));
        cast_pack.set("forge-version", serde_json::json!(loader_version));
        cast_pack.set("zapi-loader", serde_json::json!(loader));
        cast_pack.set("auto-update", serde_json::json!(auto_update));
        cast_pack.save().unwrap();
        forge::install(main_dir, cast_pack).await;
    } else if loader.eq("fabric") {
        cast_pack.set("version", serde_json::json!(version));
        cast_pack.set("fabric-loader", serde_json::json!(loader_version));
        cast_pack.set("zapi-loader", serde_json::json!(loader));
        cast_pack.set("auto-update", serde_json::json!(auto_update));
        cast_pack.save().unwrap();
        fabric::install(main_dir, cast_pack).await;
    } else {
        panic!("Unknown mod loader for modpack")
    }
}

pub async fn generate_args(main_dir: &Path, cast_pack: &mut CastPack) -> Vec<String> {
    if cast_pack.get("auto-update").is_some() {
        if cast_pack.get("auto-update").unwrap().as_bool().unwrap() {
            install(main_dir, cast_pack).await;
        }
    }
    let mut arguments: Vec<String> = Vec::new();
    let minecraft_dir = cast_pack.dir().join(".minecraft");
    create_dir_all(&minecraft_dir).unwrap();
    
    let loader = cast_pack.get("zapi-loader").unwrap().as_str().unwrap();
    if loader.eq("fabric") {
        arguments = fabric::generate_args(main_dir, cast_pack).await;
    } else if loader.eq("forge") {
        arguments = forge::generate_args(main_dir, cast_pack).await;
    } else {
        panic!("Unknown mod loader for modpack")
    }
    
    arguments
}