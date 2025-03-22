use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Settings {
    path: PathBuf,
    pub(crate) data: serde_json::Value,
}

impl Settings {
    pub fn set_data(&mut self, data: &serde_json::Value) {
        self.data = data.clone();
    }

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir().ok_or("Failed to get config directory")?;
        let config_dir = config_dir.join("cast-launcher");
        let path = config_dir.join("../settings.json");

        if path.exists() {
            let mut file = fs::File::open(&path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let data: serde_json::Value = serde_json::from_str(&content)?;
            Ok(Self { path, data })
        } else {
            let json = serde_json::json!({
                "packs_dir": config_dir.to_string_lossy(),
                "java_options": {
                    "memory": {
                        "min": "1024",
                        "max": "4096"
                    },
                    "path": "java"
                },
                "profiles": []
            });
            Ok(Self { path, data: json })
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn dir(&self) -> Option<&Path> {
        self.path.parent()
    }

    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        self.data[key] = value;
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.path, content)?;
        Ok(())
    }
}
