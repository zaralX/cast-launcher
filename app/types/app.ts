export interface AppConfig {
    launcher: LauncherConfig
    java: JavaConfig
    minecraft: MinecraftConfig
}

export interface LauncherConfig {
    language: String
    theme: String
}

export interface JavaConfig {
    java_path?: String
    min_ram: Number
    max_ram: Number
}

export interface MinecraftConfig {
    game_dir: String
}