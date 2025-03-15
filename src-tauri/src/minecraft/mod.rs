mod cast_pack_json;

use std::fs::create_dir_all;
use std::path::Path;
use crate::minecraft::cast_pack_json::CastPack;
use crate::utils;

pub async fn create_pack(main_dir: &Path, data: &mut serde_json::Value) -> Result<(), String> {
    create_dir_all(main_dir).unwrap(); // Creating launcher container folder

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

    let pack_dir = utils::create_unique_dir(main_dir, id.as_str()).unwrap();
    let mut cast_pack = CastPack::new(pack_dir);
    cast_pack.set_data(data);
    cast_pack.save().unwrap();
    
    Ok(())
}