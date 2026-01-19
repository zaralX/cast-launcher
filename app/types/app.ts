export interface AppConfig {
    launcher: LauncherConfig
    java: JavaConfig
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
