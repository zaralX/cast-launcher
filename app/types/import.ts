import type {InstanceSettings, InstanceType, Playtime} from "~/types/instance"

export type LauncherKind = "prism" | "modrinth"

export interface DetectedLauncher {
    kind: LauncherKind
    label: string
    path: string
    instances: number
}

export interface ScannedPack {
    provider: string
    projectId: string
    versionId: string
    versionName: string
    name: string
}

export interface ScannedInstance {
    folder: string
    name: string
    description: string
    minecraftVersion: string
    loader?: InstanceType
    loaderVersion?: string
    loaderLabel: string
    icon?: string
    settings: InstanceSettings
    playtime: Playtime
    pack?: ScannedPack
    blocked?: string
}

export interface ImportOptions {
    assets: boolean
    libraries: boolean
    java: boolean
    icons: boolean
    linkPacks: boolean
}

export function defaultImportOptions(): ImportOptions {
    return {
        assets: true,
        libraries: true,
        java: true,
        icons: true,
        linkPacks: true
    }
}

export interface ImportRequest {
    path: string
    folders: string[]
    options: ImportOptions
}

export type ImportStage = "shared" | "instances" | "done"

export interface CopyStats {
    files: number
    bytes: number
    skipped: number
}

export interface ImportProgress {
    source: LauncherKind
    stage: ImportStage
    step: string
    done: number
    total: number
    stats: CopyStats
}

export interface ImportedInstance {
    id: string
    name: string
    linked: boolean
}

export interface SkippedInstance {
    name: string
    reason: string
}

export interface ImportReport {
    imported: ImportedInstance[]
    skipped: SkippedInstance[]
    stats: CopyStats
    cancelled: boolean
}

const UNITS = ["Б", "КБ", "МБ", "ГБ", "ТБ"]

export function formatBytes(bytes: number): string {
    if (bytes <= 0) return "0 Б"

    let value = bytes
    let unit = 0

    while (value >= 1024 && unit < UNITS.length - 1) {
        value /= 1024
        unit++
    }

    return `${value >= 100 || unit === 0 ? Math.round(value) : value.toFixed(1)} ${UNITS[unit]}`
}
