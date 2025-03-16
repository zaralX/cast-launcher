use std::fs::create_dir_all;
use std::path::Path;
use serde_json::Value;
use crate::{utils, FABRIC_LOADERS_BY_GAME_VERSION_LINK, FABRIC_LOADER_LINK};
use crate::minecraft::cast_pack_json::CastPack;
use crate::minecraft::downloaders;
use crate::minecraft::loaders::vanilla::{download_assets, download_client_jar, download_vanilla_libraries, get_assets_json, get_client_jar_path, get_version_data};
use crate::utils::{get_absolute_path, http_get_json};

pub async fn install(main_dir: &Path, cast_pack: &mut CastPack) {
    let version = cast_pack.get("version").unwrap().as_str().unwrap();
    let mut fabric_loader: &str = cast_pack.get("fabric-loader").unwrap().as_str().unwrap();

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

    // FABRIC
    let mut fabric_version: String;
    if fabric_loader.eq("latest") {
        let fabric_loaders_link = FABRIC_LOADERS_BY_GAME_VERSION_LINK.replace("%A", version);
        let fabric_loaders = utils::http_get_json(fabric_loaders_link.as_str()).await;
        let fabric_latest = utils::get_latest_fabric_version(fabric_loaders.as_array().unwrap());
        if fabric_latest.is_none() {
            panic!("NOT FOUND LATEST VERSION")
        }
        fabric_version = fabric_latest.clone().unwrap().to_owned();
    } else {
        fabric_version = fabric_loader.to_string();
    }

    // FABRIC Getting CLIENT loader data
    let fabric_loader = get_fabric_version_data(main_dir, version, fabric_version.as_str()).await;
    if !fabric_loader.is_object() {
        panic!("NOT FOUND FABRIC LOADER {} / {}", version, fabric_loader);
    }

    // FABRIC Downloading libs
    download_fabric_libraries(main_dir, cast_pack.dir(), fabric_loader["libraries"].as_array().unwrap()).await;

    // Update installed status
    cast_pack.set("fabric-loader", serde_json::json!(fabric_version));
    cast_pack.set("installed", serde_json::json!(true));
    cast_pack.save().unwrap();
}

pub async fn generate_args(main_dir: &Path, cast_pack: &mut CastPack) -> Vec<String> {
    let mut arguments: Vec<String> = Vec::new();
    let version = cast_pack.get("version").unwrap().as_str().unwrap();
    let fabric_version = cast_pack.get("fabric-loader").unwrap().as_str().unwrap();
    let minecraft_dir = cast_pack.dir().join(".minecraft");
    create_dir_all(&minecraft_dir).unwrap();

    let version_data = get_version_data(main_dir, version).await;
    let fabric_version_data = get_fabric_version_data(main_dir, version, fabric_version).await;

    let client_jar = get_client_jar_path(main_dir, version_data["id"].as_str().unwrap());
    let main_class = fabric_version_data["mainClass"].as_str().unwrap().to_string();
    let required_java_type = version_data["javaVersion"]["component"].as_str().unwrap();

    let assets_path = main_dir.join("assets");

    let mut libs = download_vanilla_libraries(main_dir, cast_pack.dir(), version_data["libraries"].as_array().unwrap()).await;
    let fabric_libraries= download_fabric_libraries(main_dir, cast_pack.dir(), fabric_version_data["libraries"].as_array().unwrap()).await;
    libs.extend(fabric_libraries);

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

pub async fn get_fabric_version_data(main_dir: &Path, version: &str, fabric_version: &str) -> Value {
    let versions_cache_dir = main_dir.join("cache").join("fabric_versions");
    create_dir_all(&versions_cache_dir).unwrap();
    let version_file_path = versions_cache_dir.join(format!("{}-{}.json", version, fabric_version).to_string());

    let version_json;
    if version_file_path.exists() {
        version_json = utils::read_json_file(version_file_path);
    } else {
        let json: Value = http_get_json(FABRIC_LOADER_LINK.replace("%A", version).replace("%B", &*fabric_version).as_str()).await;

        version_json = json;
    }

    version_json
}

pub async fn download_fabric_libraries(main_dir: &Path, instance_dir: &Path, required_libs: &Vec<Value>) -> Vec<String> {
    let libs_path = main_dir.join("libraries");

    let mut libs: Vec<String> = Vec::new();
    let mut library_counter = 1;
    for library in required_libs {
        println!("Fabric Library {}/{}", library_counter, required_libs.iter().count().to_string());
        library_counter += 1;
        let lib_url = utils::generate_maven_url(library).unwrap().to_string();
        let lib_path = lib_url.replace(library["url"].as_str().unwrap(), "").to_owned();
        let lib_dir = libs_path.join(lib_path);
        downloaders::download_file(&*lib_url, lib_dir.as_path()).await;
        libs.push(get_absolute_path(lib_dir));
    }

    libs
}