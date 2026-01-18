use std::{fs, path::PathBuf};

use crate::config::schema::AppConfig;
use tauri::{AppHandle, Manager};

const CONFIG_FILE: &str = "config.json";

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    fs::write(path, json).map_err(|e| format!("Failed to write config: {e}"))
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {e}"))?;

    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;

    dir.push(CONFIG_FILE);
    Ok(dir)
}
