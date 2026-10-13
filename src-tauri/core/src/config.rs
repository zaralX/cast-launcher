use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::CommandResult;
use crate::fs_util::{read_json_opt, write_json_atomic};
use crate::theme::{self, CustomTheme};

pub const CONFIG_VERSION: u32 = 8;

pub const DEFAULT_THEME: &str = "depth";

pub const DEFAULT_ACCENT: &str = "theme";

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
    #[serde(default = "default_light")]
    pub theme_light: String,
    #[serde(default = "default_dark")]
    pub theme_dark: String,
    #[serde(default)]
    pub themes: Vec<CustomTheme>,
    pub dir: String,
    pub auto_update: bool,
    #[serde(default)]
    pub castpack_url: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default)]
    pub compact: bool,
    #[serde(default = "yes")]
    pub telemetry: bool,
    #[serde(default)]
    pub after_launch: AfterLaunch,
    #[serde(default)]
    pub onboarded: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AfterLaunch {
    #[default]
    Nothing,
    Hide,
    Close,
}

impl AfterLaunch {
    pub fn key(self) -> &'static str {
        match self {
            Self::Nothing => "nothing",
            Self::Hide => "hide",
            Self::Close => "close",
        }
    }
}

fn default_accent() -> String {
    DEFAULT_ACCENT.into()
}

fn default_light() -> String {
    theme::DEFAULT_LIGHT.into()
}

fn default_dark() -> String {
    theme::DEFAULT_DARK.into()
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
    #[serde(default)]
    pub jvm_args: String,
}

impl AppConfig {
    pub fn defaults(config_root: &Path) -> Self {
        Self {
            version: CONFIG_VERSION,
            launcher: LauncherConfig {
                language: "ru".into(),
                theme: DEFAULT_THEME.into(),
                theme_light: default_light(),
                theme_dark: default_dark(),
                themes: Vec::new(),
                dir: config_root.display().to_string(),
                auto_update: true,
                castpack_url: String::new(),
                accent: DEFAULT_ACCENT.into(),
                compact: false,
                telemetry: true,
                after_launch: AfterLaunch::Nothing,
                onboarded: false,
            },
            java: JavaConfig {
                java_mode: JavaMode::Auto,
                java_path: String::new(),
                min_ram: 1024,
                max_ram: 4096,
                jvm_args: String::new(),
            },
        }
    }

    pub fn normalized(mut self) -> Self {
        let launcher = &mut self.launcher;

        launcher.themes = theme::normalize(std::mem::take(&mut launcher.themes));
        launcher.theme = theme::choice(&launcher.theme, &launcher.themes, true, DEFAULT_THEME);
        launcher.theme_light = theme::choice(
            &launcher.theme_light,
            &launcher.themes,
            false,
            theme::DEFAULT_LIGHT,
        );
        launcher.theme_dark = theme::choice(
            &launcher.theme_dark,
            &launcher.themes,
            false,
            theme::DEFAULT_DARK,
        );

        self.java.jvm_args = self.java.jvm_args.trim().to_string();

        self
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

    pub fn custom_jvm_args(&self) -> Vec<String> {
        crate::launch::args::split_user_args(&self.java.jvm_args)
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
        set_in(
            &mut raw,
            "launcher",
            "accent",
            Value::String(DEFAULT_ACCENT.into()),
        );
        set_in(&mut raw, "launcher", "compact", Value::Bool(false));
    }

    if version <= 4 {
        set_in(&mut raw, "launcher", "telemetry", Value::Bool(true));
    }

    if version <= 5 {
        let value = Value::String(AfterLaunch::Nothing.key().into());
        set_in(&mut raw, "launcher", "after_launch", value);
    }

    if version <= 6 {
        set_in(&mut raw, "launcher", "onboarded", Value::Bool(true));
    }

    if version <= 7 {
        replace_in(&mut raw, "launcher", "theme", "dark", DEFAULT_THEME);
        replace_in(&mut raw, "launcher", "accent", "sky", DEFAULT_ACCENT);
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
            theme_light: string_or(launcher, "theme_light", defaults.launcher.theme_light),
            theme_dark: string_or(launcher, "theme_dark", defaults.launcher.theme_dark),
            themes: launcher
                .and_then(|launcher| launcher.get("themes"))
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| serde_json::from_value(item.clone()).ok())
                        .collect()
                })
                .unwrap_or_default(),
            dir: string_or(launcher, "dir", defaults.launcher.dir),
            auto_update: bool_or(launcher, "auto_update", defaults.launcher.auto_update),
            castpack_url: string_or(launcher, "castpack_url", defaults.launcher.castpack_url),
            accent: non_empty_string_or(launcher, "accent", defaults.launcher.accent),
            compact: bool_or(launcher, "compact", defaults.launcher.compact),
            telemetry: bool_or(launcher, "telemetry", defaults.launcher.telemetry),
            after_launch: launcher
                .and_then(|launcher| launcher.get("after_launch"))
                .and_then(|mode| serde_json::from_value(mode.clone()).ok())
                .unwrap_or(defaults.launcher.after_launch),
            onboarded: bool_or(launcher, "onboarded", defaults.launcher.onboarded),
        },
        java: JavaConfig {
            java_mode: java
                .and_then(|java| java.get("java_mode"))
                .and_then(|mode| serde_json::from_value(mode.clone()).ok())
                .unwrap_or(defaults.java.java_mode),
            java_path: string_or(java, "java_path", defaults.java.java_path),
            min_ram: number_or(java, "min_ram", defaults.java.min_ram),
            max_ram: number_or(java, "max_ram", defaults.java.max_ram),
            jvm_args: string_or(java, "jvm_args", defaults.java.jvm_args),
        },
    }
    .normalized()
}

fn set_in(raw: &mut Value, section: &str, key: &str, value: Value) {
    let entry = raw
        .as_object_mut()
        .expect("migrate is only called on objects")
        .entry(section.to_string())
        .or_insert_with(|| Value::Object(Default::default()));

    if let Some(object) = entry.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

fn replace_in(raw: &mut Value, section: &str, key: &str, from: &str, to: &str) {
    let value = raw
        .get_mut(section)
        .and_then(|section| section.get_mut(key));

    if let Some(value) = value.filter(|value| value.as_str() == Some(from)) {
        *value = Value::String(to.into());
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

        assert_eq!(
            config.launcher.catalog_url(),
            crate::castpack::catalog::DEFAULT_URL
        );

        config.launcher.castpack_url = "   ".into();
        assert_eq!(
            config.launcher.catalog_url(),
            crate::castpack::catalog::DEFAULT_URL
        );

        config.launcher.castpack_url = " https://свой.каталог/packs.json ".into();
        assert_eq!(
            config.launcher.catalog_url(),
            "https://свой.каталог/packs.json"
        );
    }

    #[test]
    fn a_config_written_before_castpack_existed_keeps_the_default_catalog() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "auto_update": true }
        }));

        assert!(config.launcher.castpack_url.is_empty());
        assert_eq!(
            config.launcher.catalog_url(),
            crate::castpack::catalog::DEFAULT_URL
        );
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
    fn v5_config_keeps_the_launcher_open_after_launch() {
        let config = migrated(json!({
            "version": 5,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "telemetry": false }
        }));

        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.launcher.after_launch, AfterLaunch::Nothing);
        assert!(!config.launcher.telemetry);
    }

    #[test]
    fn v6_config_is_considered_already_onboarded() {
        let config = migrated(json!({
            "version": 6,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "accent": "violet" }
        }));

        assert_eq!(config.version, CONFIG_VERSION);
        assert!(config.launcher.onboarded);
    }

    #[test]
    fn v7_config_moves_from_dark_and_sky_to_the_theme_defaults() {
        let config = migrated(json!({
            "version": 7,
            "launcher": { "language": "ru", "theme": "dark", "dir": "/data", "accent": "sky" }
        }));

        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.launcher.theme, DEFAULT_THEME);
        assert_eq!(config.launcher.accent, DEFAULT_ACCENT);
    }

    #[test]
    fn v7_config_keeps_a_picked_theme_and_accent() {
        let config = migrated(json!({
            "version": 7,
            "launcher": { "language": "ru", "theme": "light", "dir": "/data", "accent": "violet" }
        }));

        assert_eq!(config.launcher.theme, "light");
        assert_eq!(config.launcher.accent, "violet");
    }

    #[test]
    fn a_fresh_config_still_needs_onboarding() {
        assert!(!AppConfig::defaults(Path::new("/cfg")).launcher.onboarded);
    }

    #[test]
    fn a_finished_onboarding_survives_a_round_trip() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "onboarded": true }
        }));

        assert!(config.launcher.onboarded);

        let unfinished = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "onboarded": false }
        }));

        assert!(!unfinished.launcher.onboarded);
    }

    #[test]
    fn the_after_launch_mode_survives_a_round_trip() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "after_launch": "hide" }
        }));

        assert_eq!(config.launcher.after_launch, AfterLaunch::Hide);
    }

    #[test]
    fn an_unknown_after_launch_mode_falls_back_to_the_default_one() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "after_launch": "explode" }
        }));

        assert_eq!(config.launcher.after_launch, AfterLaunch::Nothing);
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
    fn custom_themes_survive_a_round_trip() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": {
                "theme": "system",
                "theme_light": "custom-day",
                "theme_dark": "custom-night",
                "themes": [
                    { "id": "custom-day", "name": "День", "colors": { "panel": "#ffffff" } },
                    { "id": "custom-night", "name": "Ночь", "classic": true, "colors": { "line": "#ffffff1f" } },
                    { "name": "no id" }
                ]
            }
        }));

        assert_eq!(config.launcher.theme, theme::SYSTEM);
        assert_eq!(config.launcher.theme_light, "custom-day");
        assert_eq!(config.launcher.theme_dark, "custom-night");
        assert_eq!(config.launcher.themes.len(), 2);
        assert!(config.launcher.themes[1].classic);
    }

    #[test]
    fn theme_settings_fall_back_when_their_custom_theme_is_gone() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": {
                "theme": "custom-gone",
                "theme_light": "custom-gone",
                "theme_dark": "system",
                "themes": []
            }
        }));

        assert_eq!(config.launcher.theme, DEFAULT_THEME);
        assert_eq!(config.launcher.theme_light, theme::DEFAULT_LIGHT);
        assert_eq!(config.launcher.theme_dark, theme::DEFAULT_DARK);
    }

    #[test]
    fn an_older_config_gets_the_default_system_pair() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "launcher": { "theme": "moss" }
        }));

        assert_eq!(config.launcher.theme, "moss");
        assert_eq!(config.launcher.theme_light, theme::DEFAULT_LIGHT);
        assert_eq!(config.launcher.theme_dark, theme::DEFAULT_DARK);
        assert!(config.launcher.themes.is_empty());
    }

    #[test]
    fn a_config_written_before_jvm_arguments_existed_adds_none() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "java": { "java_mode": "auto", "min_ram": 512, "max_ram": 1024 }
        }));

        assert!(config.java.jvm_args.is_empty());
        assert!(config.custom_jvm_args().is_empty());
    }

    #[test]
    fn jvm_arguments_survive_a_round_trip_trimmed() {
        let config = migrated(json!({
            "version": CONFIG_VERSION,
            "java": { "jvm_args": "  -XX:+UseG1GC -Dfile.encoding=UTF-8
        " }
        }));

        assert_eq!(config.java.jvm_args, "-XX:+UseG1GC -Dfile.encoding=UTF-8");
        assert_eq!(
            config.custom_jvm_args(),
            vec!["-XX:+UseG1GC", "-Dfile.encoding=UTF-8"]
        );
    }

    #[test]
    fn heap_args_keep_max_above_min() {
        let mut config = AppConfig::defaults(Path::new("/cfg"));
        config.java.min_ram = 4096;
        config.java.max_ram = 1024;

        assert_eq!(config.heap_args(), vec!["-Xms4096M", "-Xmx4096M"]);
    }
}
