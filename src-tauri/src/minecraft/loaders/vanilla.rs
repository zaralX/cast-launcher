use std::fmt::format;
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde_json::Value;
use crate::minecraft::cast_pack_json::CastPack;
use crate::minecraft::downloaders;
use crate::utils::{get_absolute_path, http_get_json};
use crate::{utils, VERSION_MANIFEST_LINK};

pub async fn install(main_dir: &Path, cast_pack: &mut CastPack) {
    let version = cast_pack.get("version").unwrap().as_str().unwrap();

    // Version data
    let version_data = get_version_data(main_dir, version).await;

    // Download vanilla client.jar
    download_client_jar(main_dir, &version_data).await;

    // Download vanilla libraries + natives for legacy
    download_vanilla_libraries(main_dir, cast_pack.dir(), version_data["libraries"].as_array().unwrap()).await;

    // Vanilla assets list (assets/indexes)
    let assets_json = get_assets_json(main_dir, version_data).await;
    
    // Download vanilla assets
    download_assets(main_dir, assets_json).await;
    
    // Update installed status
    cast_pack.set("installed", serde_json::json!(true));
    cast_pack.save().unwrap();
}

pub async fn generate_args(main_dir: &Path, cast_pack: &mut CastPack) -> Vec<String> {
    let mut arguments: Vec<String> = Vec::new();
    let version = cast_pack.get("version").unwrap().as_str().unwrap();
    let minecraft_dir = cast_pack.dir().join(".minecraft");
    
    let version_data = get_version_data(main_dir, version).await;

    let client_jar = get_client_jar_path(main_dir, version_data["id"].as_str().unwrap());
    let main_class = version_data["mainClass"].as_str().unwrap().to_string();
    let required_java_type = version_data["javaVersion"]["component"].as_str().unwrap();

    let assets_path = main_dir.join("assets");

    let libs = download_vanilla_libraries(main_dir, cast_pack.dir(), version_data["libraries"].as_array().unwrap()).await;
    
    // -cp Argument
    arguments.push("-cp".to_string());
    arguments.push(libs.join(";") + ";" + &*get_absolute_path(client_jar));

    // Natives path for <1.13
    if required_java_type.eq("jre-legacy") {
        let natives_dir = minecraft_dir.join("natives");
        create_dir_all(&natives_dir).unwrap();
        arguments.push(format!("-Djava.library.path={}", get_absolute_path(natives_dir)).to_string());
    }

    // Main class after -cp "" <here> OR -cp "" -Djava.library.path={} <here>
    arguments.push(main_class);

    // Any version args
    arguments.push("--version".to_string());
    arguments.push(version.to_string());
    arguments.push("--accessToken".to_string());
    arguments.push("null".to_string());
    arguments.push("--assetsDir".to_string());
    arguments.push(get_absolute_path(assets_path));
    arguments.push("--assetIndex".to_string());
    arguments.push((&version_data["assetIndex"]["id"]).as_str().unwrap().to_string());
    arguments.push("--gameDir".to_string());
    arguments.push(get_absolute_path(minecraft_dir.clone()));
    
    arguments
}

// VERSION_MANIFEST_LINK -> VERSION.URL.DATA
pub async fn get_version_data(main_dir: &Path, version: &str) -> Value {
    let versions_cache_dir = main_dir.join("cache").join("vanilla_versions");
    create_dir_all(&versions_cache_dir).unwrap();
    let version_file_path = versions_cache_dir.join(format!("{}.json", version).to_string());

    let version_json;
    if version_file_path.exists() {
         version_json = utils::read_json_file(version_file_path);
    } else {
        let manifest: Value = http_get_json(VERSION_MANIFEST_LINK).await;
        let version_url = manifest["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"].as_str().unwrap() == version)
            .unwrap()["url"]
            .as_str()
            .unwrap();

        let version_json_str = reqwest::get(version_url).await.unwrap().text().await.unwrap();
        version_json = serde_json::from_str(&version_json_str).unwrap();

        let content = serde_json::to_string_pretty(&version_json).unwrap();
        fs::write(version_file_path, content).unwrap();
    }
    
    version_json
}

pub async fn download_client_jar(main_dir: &Path, version_data: &Value) -> PathBuf {
    let client_jar = get_client_jar_path(main_dir, version_data["id"].as_str().unwrap());
    downloaders::download_file(version_data["downloads"]["client"]["url"].as_str().unwrap(), client_jar.as_path()).await;

    client_jar
}

pub async fn download_vanilla_libraries(main_dir: &Path, instance_dir: &Path, required_libs: &Vec<Value>) -> Vec<String> {
    let libs_path = main_dir.join("libraries");
    let minecraft_dir = instance_dir.join(".minecraft");

    let mut libs: Vec<String> = Vec::new();
    let mut library_counter = 1;
    for library in required_libs {
        println!("Vanilla Library {}/{}", library_counter, required_libs.iter().count().to_string());
        library_counter += 1;
        let lib_result = downloaders::download_library(&*minecraft_dir, &*libs_path, library).await;
        if lib_result.is_some() {
            libs.push(lib_result.unwrap())
        }
    }

    libs
}

pub async fn get_assets_json(main_dir: &Path, version_data: Value) -> Value {
    let assets_path = main_dir.join("assets");
    let assets_data = &version_data["assetIndex"];
    let assets_id = &assets_data["id"];
    let assets_file_name = assets_id.as_str().unwrap().to_owned() + ".json";
    let assets_file_path = &assets_path.join("indexes").join(assets_file_name);
    let assets_list: Value;
    if !&assets_file_path.exists() {
        let response = reqwest::get(assets_data["url"].as_str().unwrap())
            .await
            .expect("Error when get assets")
            .text()
            .await
            .unwrap();
        assets_list = serde_json::from_str(&response).unwrap();
        if !&assets_file_path.parent().unwrap().exists() {
            create_dir_all(&assets_file_path.parent().unwrap()).unwrap();
        }
        let mut assets_file = File::create(assets_file_path).unwrap();
        assets_file.write_all(assets_list.to_string().as_ref()).expect("Failed to save assets list");
    } else {
        let mut file = File::open(assets_file_path).unwrap();
        let mut json_string = String::new();
        file.read_to_string(&mut json_string).unwrap();
        assets_list = serde_json::from_str(&json_string).unwrap();
    }
    
    assets_list
}

pub async fn download_assets(main_dir: &Path, assets_json: Value) {
    let assets_path = main_dir.join("assets");
    let assets_objects_path = assets_path.join("objects");
    let assets_objects = assets_json["objects"].as_object().unwrap();
    let mut asset_counter = 1;
    for (key, value) in assets_objects {
        println!("Asset {}/{}", asset_counter, assets_objects.iter().count().to_string());
        asset_counter += 1;
        downloaders::download_asset(assets_objects_path.as_path(), value).await
    }
}

pub fn get_client_jar_path(main_dir: &Path, version: &str) -> PathBuf {
    main_dir.join("libraries").join("com").join("mojang").join("minecraft").join(version).join("client.jar")
}