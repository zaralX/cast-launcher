export interface AppConfig {
    version: number
    launcher: LauncherConfig
    java: JavaConfig
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
    java_path: string
    min_ram: number
    max_ram: number
}

export interface LauncherPaths {
    root: string
    configRoot: string
    instancesRoot: string
    icons: string
    libraries: string
    assets: string
    javaRuntimes: string
    logs: string
}

export type JavaSource =
    | "path"
    | "java_home"
    | "registry"
    | "system"
    | "minecraft"
    | "launcher"
    | "manual"

export interface JavaRuntime {
    path: string
    version: string
    major: number
    vendor: string
    arch: string
    os_version: string
    is_64bit: boolean
    source: JavaSource
}

export const JAVA_SOURCE_LABELS: Record<JavaSource, string> = {
    path: "PATH",
    java_home: "JAVA_HOME",
    registry: "Реестр",
    system: "Система",
    minecraft: "Minecraft",
    launcher: "Лаунчер",
    manual: "Вручную"
}
