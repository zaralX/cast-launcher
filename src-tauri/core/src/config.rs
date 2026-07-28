use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::CommandResult;
use crate::fs_util::{read_json_opt, write_json_atomic};

pub const CONFIG_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub launcher: LauncherConfig,
    pub java: JavaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub language: String,
    pub theme: String,
    pub dir: String,
    pub auto_update: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JavaMode {
    #[default]
    Auto,
    System,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaConfig {
    pub java_mode: JavaMode,
    #[serde(default)]
    pub java_path: String,
    pub min_ram: u32,
    pub max_ram: u32,
}

impl AppConfig {
    pub fn defaults(config_root: &Path) -> Self {
        Self {
            version: CONFIG_VERSION,
            launcher: LauncherConfig {
                language: "ru".into(),
                theme: "dark".into(),
                dir: config_root.display().to_string(),
                auto_update: true,
            },
            java: JavaConfig {
                java_mode: JavaMode::Auto,
                java_path: String::new(),
                min_ram: 1024,
                max_ram: 4096,
            },
        }
    }

    pub fn manual_java_path(&self) -> Option<&str> {
        let path = self.java.java_path.trim();
        (!path.is_empty()).then_some(path)
    }

    pub fn heap_args(&self) -> Vec<String> {
        let min = self.java.min_ram.max(1);
        let max = self.java.max_ram.max(min);
        vec![format!("-Xms{min}M"), format!("-Xmx{max}M")]
    }
}

pub async fn load(config_root: &Path, config_file: &Path) -> CommandResult<AppConfig> {
    let raw: Option<Value> = read_json_opt(config_file).await;
    let defaults = AppConfig::defaults(config_root);

    let config = match raw {
        Some(raw) => merge(defaults, migrate(raw)),
        None => defaults,
    };

    save(config_file, &config).await?;

    Ok(config)
}

pub async fn save(config_file: &Path, config: &AppConfig) -> CommandResult<()> {
    write_json_atomic(config_file, config).await
}

fn migrate(mut raw: Value) -> Value {
    if !raw.is_object() {
        return Value::Object(Default::default());
    }

    let version = raw.get("version").and_then(Value::as_u64).unwrap_or(1);

    if version <= 1 {
        set_in(&mut raw, "launcher", "auto_update", Value::Bool(true));
    }

    if version <= 2 {
        let has_manual_path = raw
            .get("java")
            .and_then(|java| java.get("java_path"))
            .and_then(Value::as_str)
            .is_some_and(|path| !path.trim().is_empty());

        let mode = if has_manual_path { "manual" } else { "auto" };
        set_in(&mut raw, "java", "java_mode", Value::String(mode.into()));
    }

    raw["version"] = Value::from(CONFIG_VERSION);
    raw
}

fn merge(defaults: AppConfig, raw: Value) -> AppConfig {
    let launcher = raw.get("launcher");
    let java = raw.get("java");

    AppConfig {
        version: CONFIG_VERSION,
        launcher: LauncherConfig {
            language: string_or(launcher, "language", defaults.launcher.language),
            theme: string_or(launcher, "theme", defaults.launcher.theme),
            dir: string_or(launcher, "dir", defaults.launcher.dir),
            auto_update: bool_or(launcher, "auto_update", defaults.launcher.auto_update),
        },
        java: JavaConfig {
            java_mode: java
                .and_then(|java| java.get("java_mode"))
                .and_then(|mode| serde_json::from_value(mode.clone()).ok())
                .unwrap_or(defaults.java.java_mode),
            java_path: string_or(java, "java_path", defaults.java.java_path),
            min_ram: number_or(java, "min_ram", defaults.java.min_ram),
            max_ram: number_or(java, "max_ram", defaults.java.max_ram),
        },
    }
}

fn set_in(raw: &mut Value, section: &str, key: &str, value: Value) {
    let entry = raw
        .as_object_mut()
        .expect("migrate вызывается только для объектов")
        .entry(section.to_string())
        .or_insert_with(|| Value::Object(Default::default()));

    if let Some(object) = entry.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

fn string_or(section: Option<&Value>, key: &str, fallback: String) -> String {
    section
        .and_then(|section| section.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or(fallback)
}

fn bool_or(section: Option<&Value>, key: &str, fallback: bool) -> bool {
    section
        .and_then(|section| section.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn number_or(section: Option<&Value>, key: &str, fallback: u32) -> u32 {
    section
        .and_then(|section| section.get(key))
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn migrated(raw: Value) -> AppConfig {
        merge(AppConfig::defaults(Path::new("/cfg")), migrate(raw))
    }

    #[test]
    fn v1_config_gains_auto_update_and_java_mode() {
        let config = migrated(json!({
            "launcher": { "language": "en", "theme": "light", "dir": "/data" },
            "java": { "min_ram": 2048, "max_ram": 8192 }
        }));

        assert_eq!(config.version, CONFIG_VERSION);
        assert!(config.launcher.auto_update);
        assert_eq!(config.java.java_mode, JavaMode::Auto);
        assert_eq!(config.launcher.language, "en");
        assert_eq!(config.java.max_ram, 8192);
    }

    #[test]
    fn v2_config_with_manual_path_switches_to_manual_mode() {
        let config = migrated(json!({
            "version": 2,
            "java": { "java_path": "C:\\jdk\\bin\\java.exe" }
        }));

        assert_eq!(config.java.java_mode, JavaMode::Manual);
        assert_eq!(config.manual_java_path(), Some("C:\\jdk\\bin\\java.exe"));
    }

    #[test]
    fn current_config_survives_round_trip() {
        let config = migrated(json!({
            "version": 3,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "auto_update": false },
            "java": { "java_mode": "system", "java_path": "", "min_ram": 512, "max_ram": 1024 }
        }));

        assert!(!config.launcher.auto_update);
        assert_eq!(config.java.java_mode, JavaMode::System);
        assert_eq!(config.manual_java_path(), None);
    }

    #[test]
    fn garbage_sections_fall_back_to_defaults() {
        let config = migrated(json!({ "version": 3, "launcher": 42, "java": "nope" }));

        assert_eq!(config.launcher.language, "ru");
        assert_eq!(config.java.min_ram, 1024);
    }

    #[test]
    fn heap_args_keep_max_above_min() {
        let mut config = AppConfig::defaults(Path::new("/cfg"));
        config.java.min_ram = 4096;
        config.java.max_ram = 1024;

        assert_eq!(config.heap_args(), vec!["-Xms4096M", "-Xmx4096M"]);
    }
}
