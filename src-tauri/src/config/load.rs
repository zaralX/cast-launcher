use std::{
    fs,
    path::PathBuf,
};

use tauri::{AppHandle, Manager};
use crate::config::schema::AppConfig;

const CONFIG_FILE: &str = "config.json";

pub fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path(app)?;

    if !path.exists() {
        let default = AppConfig::default();
        save_default(&path, &default)?;
        return Ok(default);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config: {e}"))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {e}"))
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {e}"))?;

    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create config dir: {e}"))?;

    dir.push(CONFIG_FILE);
    Ok(dir)
}

fn save_default(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize default config: {e}"))?;

    fs::write(path, json)
        .map_err(|e| format!("Failed to write default config: {e}"))
}
