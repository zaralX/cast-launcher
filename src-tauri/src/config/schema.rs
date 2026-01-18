use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub launcher: LauncherConfig,
    pub java: JavaConfig,
    pub minecraft: MinecraftConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LauncherConfig {
    pub language: String,
    pub theme: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JavaConfig {
    pub java_path: Option<String>,
    pub min_ram: u32,
    pub max_ram: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MinecraftConfig {
    pub game_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            launcher: LauncherConfig {
                language: "ru".into(),
                theme: "dark".into(),
            },
            java: JavaConfig {
                java_path: None,
                min_ram: 2048,
                max_ram: 4096,
            },
            minecraft: MinecraftConfig {
                game_dir: "minecraft".into(),
            },
        }
    }
}
