use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::CommandResult;
use crate::fs_util::{read_json_opt, write_json_atomic};

pub const CONFIG_VERSION: u32 = 5;

pub const DEFAULT_ACCENT: &str = "sky";

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
    #[serde(default)]
    pub castpack_url: String,
    /// Имя палитры Nuxt UI, из которой берётся основной цвет интерфейса.
    #[serde(default = "default_accent")]
    pub accent: String,
    /// Компактный режим интерфейса.
    #[serde(default)]
    pub compact: bool,
    /// Отправка анонимной статистики использования.
    #[serde(default = "yes")]
    pub telemetry: bool,
}

fn default_accent() -> String {
    DEFAULT_ACCENT.into()
}

fn yes() -> bool {
    true
}

impl LauncherConfig {
    pub fn catalog_url(&self) -> &str {
        let url = self.castpack_url.trim();

        match url.is_empty() {
            true => crate::castpack::catalog::DEFAULT_URL,
            false => url,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JavaMode {
    #[default]
    Auto,
    System,
    Manual,
}

impl JavaMode {
    pub fn key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::System => "system",
            Self::Manual => "manual",
        }
    }
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
                castpack_url: String::new(),
                accent: DEFAULT_ACCENT.into(),
                compact: false,
                telemetry: true,
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

    if version <= 3 {
        set_in(&mut raw, "launcher", "accent", Value::String(DEFAULT_ACCENT.into()));
        set_in(&mut raw, "launcher", "compact", Value::Bool(false));
    }

    if version <= 4 {
        set_in(&mut raw, "launcher", "telemetry", Value::Bool(true));
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
            castpack_url: string_or(launcher, "castpack_url", defaults.launcher.castpack_url),
            accent: non_empty_string_or(launcher, "accent", defaults.launcher.accent),
            compact: bool_or(launcher, "compact", defaults.launcher.compact),
            telemetry: bool_or(launcher, "telemetry", defaults.launcher.telemetry),
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

fn non_empty_string_or(section: Option<&Value>, key: &str, fallback: String) -> String {
    let value = string_or(section, key, fallback.clone());

    match value.trim().is_empty() {
        true => fallback,
        false => value.trim().to_string(),
    }
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
    fn an_empty_catalog_url_falls_back_to_the_built_in_one() {
        let mut config = AppConfig::defaults(Path::new("/cfg"));

        assert_eq!(config.launcher.catalog_url(), crate::castpack::catalog::DEFAULT_URL);

        config.launcher.castpack_url = "   ".into();
        assert_eq!(config.launcher.catalog_url(), crate::castpack::catalog::DEFAULT_URL);

        config.launcher.castpack_url = " https://свой.каталог/packs.json ".into();
        assert_eq!(config.launcher.catalog_url(), "https://свой.каталог/packs.json");
    }

    #[test]
    fn a_config_written_before_castpack_existed_keeps_the_default_catalog() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "auto_update": true }
        }));

        assert!(config.launcher.castpack_url.is_empty());
        assert_eq!(config.launcher.catalog_url(), crate::castpack::catalog::DEFAULT_URL);
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
    fn v3_config_gains_the_default_appearance() {
        let config = migrated(json!({
            "version": 3,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "auto_update": true }
        }));

        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.launcher.accent, DEFAULT_ACCENT);
        assert!(!config.launcher.compact);
    }

    #[test]
    fn v4_config_gains_telemetry_turned_on() {
        let config = migrated(json!({
            "version": 4,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "accent": "violet" }
        }));

        assert_eq!(config.version, CONFIG_VERSION);
        assert!(config.launcher.telemetry);
    }

    #[test]
    fn telemetry_stays_off_once_it_was_turned_off() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "telemetry": false }
        }));

        assert!(!config.launcher.telemetry);
    }

    #[test]
    fn a_blank_accent_falls_back_to_the_default_one() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "accent": "   ", "compact": true }
        }));

        assert_eq!(config.launcher.accent, DEFAULT_ACCENT);
        assert!(config.launcher.compact);
    }

    #[test]
    fn current_config_survives_round_trip() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": {
                "language": "ru", "theme": "dark", "dir": "/data", "auto_update": false,
                "accent": "violet", "compact": true, "telemetry": true
            },
            "java": { "java_mode": "system", "java_path": "", "min_ram": 512, "max_ram": 1024 }
        }));

        assert!(!config.launcher.auto_update);
        assert!(config.launcher.telemetry);
        assert_eq!(config.launcher.accent, "violet");
        assert!(config.launcher.compact);
        assert_eq!(config.java.java_mode, JavaMode::System);
        assert_eq!(config.manual_java_path(), None);
    }

    #[test]
    fn garbage_sections_fall_back_to_defaults() {
        let config = migrated(json!({ "version": CONFIG_VERSION, "launcher": 42, "java": "nope" }));

        assert_eq!(config.launcher.language, "ru");
        assert_eq!(config.launcher.accent, DEFAULT_ACCENT);
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
