export const CONFIG_VERSION = 2

export interface AppConfig {
    launcher: LauncherConfig
    java: JavaConfig
    version: number
}

export interface LauncherConfig {
    language: string
    theme: string
    dir: string
    auto_update: boolean
}

export interface JavaConfig {
    java_path?: string
    min_ram: number
    max_ram: number
}
