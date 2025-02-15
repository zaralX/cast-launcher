use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct JavaMemory {
    pub min: String,
    pub max: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct JavaOptions {
    pub memory: JavaMemory,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Profile {
    pub username: String,
    pub selected: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Settings {
    pub packs_dir: String,
    pub java_options: JavaOptions,
    pub profiles: Vec<Profile>,
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
