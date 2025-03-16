mod cast_pack_json;
mod loaders;
mod downloaders;

use std::fs;
use std::fs::create_dir_all;
use std::path::Path;
use std::process::Command;
use log::warn;
use serde_json::Value;
use crate::minecraft::cast_pack_json::CastPack;
use crate::utils;

pub async fn create_pack(main_dir: &Path, data: &mut serde_json::Value) -> Result<(), String> {
    let instances_dir = main_dir.join("instances");
    create_dir_all(&instances_dir).unwrap(); // Creating required folders

    let id = data["id"].as_str().ok_or("Missing id field in pack data")?.to_string();
    let name = data["name"].as_str().ok_or("Missing name field in pack data")?.to_string();
    let _type = data["type"].as_str().ok_or("Missing type field in pack data")?.to_string();

    match _type.as_str() {
        "vanilla" => {
            data["version"]
                .as_str()
                .ok_or("Missing 'version' field")?;
        }
        "fabric" => {
            data["fabric-loader"]
                .as_str()
                .ok_or("Missing 'fabric-loader' field")?;
            data["version"]
                .as_str()
                .ok_or("Missing 'version' field")?;
        }
        "modrinth" => {
            data["modrinth-project-id"]
                .as_str()
                .ok_or("Missing 'modrinth-project-id' field")?;
            data["modrinth-project-version"]
                .as_str()
                .ok_or("Missing 'modrinth-project-version' field")?;
        }
        _ => return Err(format!("UNKNOWN PACK TYPE {:?}", _type)),
    }

    data["castPackVersion"] = serde_json::json!("1");
    data["installed"] = serde_json::json!(false);

    let pack_dir = utils::create_unique_dir(instances_dir.as_path(), id.as_str()).unwrap();

    let mut cast_pack = CastPack::new(pack_dir);
    cast_pack.set_data(data);
    cast_pack.save().unwrap();
    
    Ok(())
}

pub async fn install_pack(main_dir: &Path, id: &str) {
    let mut cast_pack = get_cast_pack(main_dir, id);;
    
    if cast_pack.get("type").unwrap().eq("vanilla") {
        loaders::vanilla::install(main_dir, &mut cast_pack).await;
    } else if cast_pack.get("type").unwrap().eq("fabric") {
        loaders::fabric::install(main_dir, &mut cast_pack).await;
    } else {
        panic!("UNKNOWN CAST-PACK TYPE: {}", cast_pack.get("type").unwrap())
    }
}

pub async fn run_pack(main_dir: &Path, id: &str, java: &str) {
    let mut cast_pack: CastPack = get_cast_pack(main_dir, id);

    if cast_pack.get("type").unwrap().eq("vanilla") {
        let args = loaders::vanilla::generate_args(main_dir, &mut cast_pack).await;
        println!("Launch args: {}", args.join(" ").as_str());
        let mut command = Command::new(java);
        command.args(args);
        command.current_dir(cast_pack.dir().join(".minecraft"));
        command.spawn().expect("Error when Minecraft start.");
    } else if cast_pack.get("type").unwrap().eq("fabric") {
        let args = loaders::fabric::generate_args(main_dir, &mut cast_pack).await;
        println!("Launch args: {}", args.join(" ").as_str());
        let mut command = Command::new(java);
        command.args(args);
        command.current_dir(cast_pack.dir().join(".minecraft"));
        command.spawn().expect("Error when Minecraft start.");
    } else {
        panic!("UNKNOWN CAST-PACK TYPE: {}", cast_pack.get("type").unwrap())
    }
}

pub fn get_cast_pack(main_dir: &Path, id: &str) -> CastPack {
    let instances_dir = main_dir.join("instances");
    if !instances_dir.exists() {
        panic!("Not found instances folder")
    }
    
    for entry in fs::read_dir(instances_dir).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();
        let cast_pack_path = entry_path.join("cast-pack.json");
        if !cast_pack_path.exists() {
            warn!("Not found cast pack in: {}", cast_pack_path.display());
            continue
        }

        let mut _cast_pack = CastPack::new(entry_path);
        let mut _cast_pack = _cast_pack.load().unwrap();

        if _cast_pack.get("id").unwrap().as_str().unwrap().eq(id) {
            return _cast_pack.clone();
        }
    }

    panic!("Not found instance with id: {}", id)
}

pub fn get_packs(main_dir: &Path) -> Vec<Value> {
    let instances_dir = main_dir.join("instances");
    create_dir_all(&instances_dir).unwrap();
    
    let mut packs: Vec<Value> = Vec::new();

    for entry in fs::read_dir(instances_dir).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();
        let cast_pack_path = entry_path.join("cast-pack.json");
        if !cast_pack_path.exists() {
            warn!("Not found cast pack in: {}", cast_pack_path.display());
            continue
        }

        let mut _cast_pack = CastPack::new(entry_path);
        let mut _cast_pack = _cast_pack.load().unwrap();
        
        let folder_name = cast_pack_path.file_name().and_then(|s| s.to_str()).unwrap();

        let pack = serde_json::json!({
            "cast-pack": _cast_pack.data,
            "folder": folder_name
        });
        
        packs.push(pack)
    }
    
    packs
}