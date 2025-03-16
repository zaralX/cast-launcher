use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct CastPack {
    path: PathBuf,
    pub(crate) data: serde_json::Value,
}

impl CastPack {
    pub fn new(path: PathBuf) -> Self {
        let path = path.join("cast-pack.json");
        Self {
            path,
            data: serde_json::json!({
                "castPackVersion": "1",
                "name": "undefined",
                "id": "undefined",
                "type": "undefined",
                "installed": false
            }),
        }
    }
    
    pub fn set_data(&mut self, data: &serde_json::Value) {
        self.data = data.clone();
    }

    pub fn load(&mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new(&self.path);
        if path.exists() {
            let mut file = fs::File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let data: serde_json::Value = serde_json::from_str(&content)?;
            return Ok(Self { path: path.to_path_buf(), data });
        }
        Ok(Self::new(path.to_path_buf()))
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }
    
    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap()
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