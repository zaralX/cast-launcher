export interface AppConfig {
    launcher: LauncherConfig
    java: JavaConfig
    minecraft: MinecraftConfig
}

export interface LauncherConfig {
    language: string
    theme: string
    dir: string
}

export interface JavaConfig {
    java_path?: string
    min_ram: number
    max_ram: number
}

export interface MinecraftConfig {
    game_dir: string
}