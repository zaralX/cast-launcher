export const CONFIG_VERSION = 3

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

export type JavaMode = "auto" | "system" | "manual"

export interface JavaConfig {
    java_mode: JavaMode
    java_path?: string
    min_ram: number
    max_ram: number
}

export type JavaSource = "path" | "java_home" | "registry" | "system" | "minecraft" | "manual"

export interface JavaRuntime {
    path: string
    version: string
    major: number
    vendor: string
    arch: string
    is_64bit: boolean
    source: JavaSource
}

export const JAVA_SOURCE_LABELS: Record<JavaSource, string> = {
    path: "PATH",
    java_home: "JAVA_HOME",
    registry: "Реестр",
    system: "Система",
    minecraft: "Minecraft",
    manual: "Вручную"
}
