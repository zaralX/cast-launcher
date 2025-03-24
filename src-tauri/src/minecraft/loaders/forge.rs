use std::fs;
use std::fs::{create_dir_all, File};
use std::path::Path;
use serde_json::Value;
use crate::{utils, FORGE_INSTALLER_LINK, FORGE_LIST_LINK};
use crate::minecraft::cast_pack_json::CastPack;
use crate::minecraft::downloaders;
use crate::minecraft::loaders::vanilla::{download_assets, download_client_jar, download_vanilla_libraries, get_assets_json, get_client_jar_path, get_version_data};
use crate::utils::{get_absolute_path, http_get_json};

pub async fn install(main_dir: &Path, cast_pack: &mut CastPack) {
    let version = cast_pack.get("version").unwrap().as_str().unwrap();
    let forge_version: &str = cast_pack.get("forge-version").unwrap().as_str().unwrap();

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

    // FORGE
    let forge_version_str: String;
    if forge_version.eq("latest") {
        let forge_versions = http_get_json(FORGE_LIST_LINK).await;
        let forge_latest = forge_versions["promos"][version.to_string() + "-latest"].as_str();
        if forge_latest.is_none() {
            panic!("NOT FOUND LATEST FORGE VERSION")
        }
        forge_version_str = forge_latest.clone().unwrap().to_owned();
    } else {
        forge_version_str = forge_version.to_string();
    }
    let forge_id = format!("{}-{}", version, forge_version_str);

    // FORGE Downloading installer
    let forge_installer_link = FORGE_INSTALLER_LINK.replace("%A", version).replace("%B", &forge_version_str);
    let forge_installer_link = forge_installer_link.as_str();
    download_forge_installer(main_dir, forge_installer_link, forge_id.clone()).await;

    // Install Forge OR get forge version data
    let forge_data_id = format!("{}-forge-{}", version, forge_version_str);
    let forge_data_path = main_dir.join("cache").join("forge").join("output").join("versions").join(&forge_data_id).join(forge_data_id + ".json");
    if !forge_data_path.exists() {
        run_forge_installer(main_dir, forge_id).await;
    }
    let forge_data = utils::read_json_file(forge_data_path);

    // Update installed status
    cast_pack.set("forge-version", serde_json::json!(forge_version_str));
    cast_pack.set("installed", serde_json::json!(true));
    cast_pack.save().unwrap();
}

pub async fn generate_args(main_dir: &Path, cast_pack: &mut CastPack) -> Vec<String> {
    let mut arguments: Vec<String> = Vec::new();
    let version = cast_pack.get("version").unwrap().as_str().unwrap();
    let forge_version = cast_pack.get("forge-version").unwrap().as_str().unwrap();
    let minecraft_dir = cast_pack.dir().join(".minecraft");
    create_dir_all(&minecraft_dir).unwrap();

    let version_data = get_version_data(main_dir, version).await;
    let forge_data_id = format!("{}-forge-{}", version, forge_version);
    let forge_data_path = main_dir.join("cache").join("forge").join("output").join("versions").join(&forge_data_id).join(forge_data_id + ".json");
    let forge_version_data = utils::read_json_file(forge_data_path);

    let client_jar = get_client_jar_path(main_dir, version_data["id"].as_str().unwrap());
    // let client_jar = main_dir.join("cache").join("forge").join("output").join("versions").join(version).join(version.to_string() + ".jar");
    let main_class = forge_version_data["mainClass"].as_str().unwrap().to_string();
    let required_java_type = version_data["javaVersion"]["component"].as_str().unwrap();

    let assets_path = main_dir.join("assets");

    let mut libs = download_vanilla_libraries(main_dir, cast_pack.dir(), version_data["libraries"].as_array().unwrap()).await;
    let forge_libraries = download_forge_libraries(main_dir, cast_pack.dir(), forge_version_data["libraries"].as_array().unwrap()).await;
    libs.extend(forge_libraries);

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

    // <1.13
    if required_java_type.eq("jre-legacy") {
        arguments.push("--tweakClass".to_string());
        arguments.push("net.minecraftforge.fml.common.launcher.FMLTweaker".to_string());
    } else {
        // >=1.13
        if let Some(game_args) = forge_version_data["arguments"]["game"].as_array() {
            let game_args: Vec<String> = game_args
                .iter()
                .filter_map(|arg| arg.as_str().map(|s| s.to_string()))
                .collect();

            arguments.extend(game_args);
        } else {
            println!("Game args not exists");
        }
        if let Some(jvm_args) = forge_version_data["arguments"]["jvm"].as_array() {
            let libs_dir = get_absolute_path(main_dir.join("libraries"));
            let jvm_args: Vec<String> = jvm_args
                .iter()
                .filter_map(|arg| arg.as_str().map(|s| s.to_string()
                    .replace("${library_directory}", &libs_dir)
                    .replace("${version_name}", &version)
                    .replace("${classpath_separator}", ";") // TODO detect for linux/windows/macos
                ))
                .collect();

            arguments.extend(jvm_args);
        } else {
            println!("Jvm args not exists");
        }
    }

    arguments
}

pub async fn download_forge_installer(main_dir: &Path, link: &str, id: String) {
    let installer_path = main_dir.join("cache").join("forge").join("installers").join(id + ".jar");
    create_dir_all(installer_path.parent().unwrap()).unwrap();
    downloaders::download_file(link, installer_path.as_path()).await;
}

pub async fn run_forge_installer(main_dir: &Path, id: String) {
    let installer_path = main_dir.join("cache").join("forge").join("installers").join(id + ".jar");
    let install_output = main_dir.join("cache").join("forge").join("output");
    let launcher_profiles_path = install_output.join("launcher_profiles.json");

    create_dir_all(&install_output).unwrap();
    create_dir_all(installer_path.clone().parent().unwrap()).unwrap();
    File::create(launcher_profiles_path).unwrap(); // Fix idiot installer

    let full_installer_path = utils::get_absolute_path(installer_path);
    let args = vec!["-jar", full_installer_path.as_str(), "--installClient"];

    println!("Running Forge installer...");
    std::process::Command::new("java")
        .args(args)
        .current_dir(&install_output)
        .spawn()
        .expect("Failed to execute Forge installer")
        .wait()
        .expect("Forge installer process failed");

    println!("Moving libraries");
    let from = install_output.join("libraries");
    let to = main_dir.join("libraries");
    utils::move_files(from.clone(), to).unwrap();
    println!("Deleting output libraries");
    fs::remove_dir_all(from).unwrap();
}

pub async fn download_forge_libraries(main_dir: &Path, instance_dir: &Path, required_libs: &Vec<Value>) -> Vec<String> {
    let libs_path = main_dir.join("libraries");

    let mut libs: Vec<String> = Vec::new();
    let mut library_counter = 1;
    for library in required_libs {
        println!("Forge Library {}/{}", library_counter, required_libs.iter().count().to_string());
        library_counter += 1;
        let lib_url = library["downloads"]["artifact"]["url"].as_str().unwrap();
        let lib_path = library["downloads"]["artifact"]["path"].as_str().unwrap();
        let lib_dir = libs_path.join(lib_path);
        downloaders::download_file(&*lib_url, lib_dir.as_path()).await;
        libs.push(get_absolute_path(lib_dir));
    }

    libs
}