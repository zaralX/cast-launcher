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
    accent: string
    compact: boolean
    telemetry: boolean
    after_launch: AfterLaunch
}

export type AfterLaunch = "nothing" | "hide" | "close"

export const AFTER_LAUNCH_OPTIONS: { value: AfterLaunch, label: string, hint: string }[] = [
    {value: "nothing", label: "Оставить открытым", hint: "Лаунчер никуда не денется."},
    {value: "hide", label: "Скрыть лаунчер", hint: "Окно вернётся, когда игра закроется."},
    {value: "close", label: "Закрыть лаунчер", hint: "Лаунчер доработает в фоне и выйдет вместе с игрой."}
]

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
