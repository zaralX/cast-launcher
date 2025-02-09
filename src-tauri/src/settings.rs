use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
struct JavaMemory {
    min: String,
    max: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JavaOptions {
    memory: JavaMemory,
    path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Profile {
    username: String,
    selected: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Settings {
    packs_dir: String,
    java_options: JavaOptions,
    profiles: Vec<Profile>,
}

impl Settings {
    fn default() -> Self {
        Self {
            packs_dir: "".to_string(),
            java_options: JavaOptions {
                memory: JavaMemory {
                    min: "1024".to_string(),
                    max: "4096".to_string(),
                },
                path: "".to_string(),
            },
            profiles: vec![Profile {
                username: "".to_string(),
                selected: true,
            }],
        }
    }
}

pub(crate) fn load_settings() -> Settings {
    let data = fs::read_to_string("settings.json").unwrap_or_else(|_| {
        serde_json::to_string_pretty(&Settings::default()).unwrap()
    });

    serde_json::from_str(&data).unwrap_or_else(|_| Settings::default())
}

pub(crate) fn save_settings(settings: &Settings) {
    if let Ok(data) = serde_json::to_string_pretty(settings) {
        let _ = fs::write("settings.json", data);
    }
}
