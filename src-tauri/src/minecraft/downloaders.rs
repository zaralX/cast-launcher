use std::fs;
use std::fs::create_dir_all;
use std::path::Path;
use crate::{utils, ASSETS_LINK};

pub async fn download_library(instance_dir: &Path, libs_dir: &Path, library: &serde_json::Value) -> Option<String> {
    let rules = &library["rules"].as_array();
    if rules.is_some() {
        let rules = rules.unwrap();
        let mut skip = false;
        for rule in rules {
            let rule_action = rule["action"].as_str();
            let rule_os = rule["os"].as_object();
            if rule_os.is_some() {
                let os_name = &rule_os?.get("name");
                let os_arch = &rule_os?.get("arch");
                if os_name.is_some() {
                    if utils::is_current_os(&os_name.unwrap().as_str()?) {
                        if rule_action?.to_string().eq("disallow") {
                            println!("Skipped lib by OS NAME [DISALLOW]: {}", library.to_string());
                            skip = true;
                            continue
                        }
                    } else {
                        // if allowed not for this os
                        println!("Skipped lib by OS NAME [ALLOW ONLY]: {}", library.to_string());
                        skip = true;
                        continue
                    }
                }
                if os_arch.is_some() {
                    if utils::is_current_arch(&os_arch.unwrap().as_str()?) {
                        if rule_action?.to_string().eq("disallow") {
                            println!("Skipped lib by OS ARCH [DISALLOW]: {}", library.to_string());
                            skip = true;
                            continue
                        }
                    } else {
                        // if allowed not for this os
                        println!("Skipped lib by OS ARCH [ALLOW ONLY]: {}", library.to_string());
                        skip = true;
                        continue
                    }
                }
            } else if rule_action.is_some() && rule.as_object()?.iter().count() == 1 {
                if rule_action?.eq("allow") {
                    skip = false;
                    break;
                } else if rule_action?.eq("disallow") {
                    skip = true;
                    break;
                }
            }
        }
    }

    let lib_artifact = &library["downloads"]["artifact"];
    let lib_classifier = &library["downloads"]["classifiers"]["natives-windows"];
    if !lib_classifier.is_null() {
        let lib_path = libs_dir.join(&lib_classifier["path"].as_str()?);
        download_file(lib_classifier["url"].as_str()?, lib_path.as_path()).await;
        let natives_dir = instance_dir.join("natives");
        if !natives_dir.exists() {
            create_dir_all(&natives_dir).unwrap();
        }
        utils::extract_jar(lib_path, natives_dir).unwrap();
        return None;
    } else if lib_artifact.is_null() {
        println!("NOT FOUNT ARTIFACT OR CLASSIFIER FOR WINDOWS:");
        println!("{}", library.to_string());
        return None;
    }
    let lib_path = libs_dir.join(&lib_artifact["path"].as_str()?);
    download_file(lib_artifact["url"].as_str()?, lib_path.as_path()).await;
    Some(utils::get_absolute_path(lib_path))
}

pub async fn download_asset(assets_objects_dir: &Path, library: &serde_json::Value) {
    let hash = library["hash"].as_str().unwrap();
    let folder = &hash[..2];
    let object_folder = assets_objects_dir.join(folder);
    if !&object_folder.exists() {
        create_dir_all(&object_folder).unwrap();
    }
    let object_link = ASSETS_LINK.replace("%A", folder).replace("%B", hash);
    let object_file_path = object_folder.join(hash);
    download_file(object_link.as_str(), object_file_path.as_path()).await;
}

pub async fn download_file(url: &str, dir: &Path) {
    if !&dir.parent().unwrap().exists() {
        create_dir_all(&dir.parent().unwrap()).unwrap();
    }
    if !&dir.exists() {
        println!("Downloading: {}", &dir.to_string_lossy());
        let jar_data = reqwest::get(url).await.unwrap().bytes().await.unwrap();
        fs::write(&dir, jar_data).unwrap();
    } else {
        println!("{} exists! Skipped download.", &dir.to_string_lossy());
    }
}