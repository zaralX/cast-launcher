import type {JavaMode} from "~/types/app"
import type {BlockedFile} from "~/types/catalog"
import type {CastPackSource} from "~/types/castpack"

export type InstanceType = "vanilla" | "fabric" | "forge" | "neoforge"

export const INSTANCE_TYPE_LABELS: Record<InstanceType, string> = {
    vanilla: "Vanilla",
    fabric: "Fabric",
    forge: "Forge",
    neoforge: "NeoForge"
}

export interface InstanceSettings {
    overrideMemory: boolean
    minRam: number
    maxRam: number
    overrideJava: boolean
    javaMode: JavaMode
    javaPath: string
}

export function emptyInstanceSettings(): InstanceSettings {
    return {
        overrideMemory: false,
        minRam: 0,
        maxRam: 0,
        overrideJava: false,
        javaMode: "auto",
        javaPath: ""
    }
}

export type PackProvider = "modrinth" | "curseforge"

export const PACK_PROVIDER_LABELS: Record<PackProvider, string> = {
    modrinth: "Modrinth",
    curseforge: "CurseForge"
}

export interface PackSource {
    provider: PackProvider
    projectId: string
    versionId: string
    versionNumber: string
    fileUrl: string
    fileName: string
    fileSha1?: string
    fileSize?: number
}

export type LocalPackKind = "modrinth" | "curseforge" | "multimc"

export const LOCAL_PACK_KIND_LABELS: Record<LocalPackKind, string> = {
    modrinth: "Modrinth (.mrpack)",
    curseforge: "CurseForge",
    multimc: "MultiMC / Prism"
}

export interface LocalPackSource {
    kind: LocalPackKind
    name: string
    version: string
}

export interface Playtime {
    totalSeconds: number
    lastSeconds: number
    lastPlayedAt: number
}

export function emptyPlaytime(): Playtime {
    return {totalSeconds: 0, lastSeconds: 0, lastPlayedAt: 0}
}

export function formatPlaytime(seconds: number): string {
    if (!seconds || seconds < 0) return ""
    if (seconds < 60) return "меньше минуты"

    const days = Math.floor(seconds / 86400)
    const hours = Math.floor(seconds / 3600) % 24
    const minutes = Math.floor(seconds / 60) % 60

    const parts: string[] = []

    if (days) parts.push(`${days} д`)
    if (hours) parts.push(`${hours} ч`)
    if (minutes && !days) parts.push(`${minutes} мин`)

    return parts.join(" ")
}

export function formatLastPlayed(millis: number): string {
    if (!millis) return ""

    return new Date(millis).toLocaleString("ru-RU", {
        day: "2-digit",
        month: "2-digit",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit"
    })
}

export interface Instance {
    id: string
    name: string
    description: string
    minecraftVersion: string
    icon: string
    type: InstanceType
    installed: boolean
    version: number
    loaderVersion?: string
    customId?: string
    pack?: PackSource
    castpack?: CastPackSource
    localPack?: LocalPackSource
    settings: InstanceSettings
    playtime: Playtime
}

export interface InstanceLogFile {
    name: string
    size: number
    modified: number
}

export interface GameLogLine {
    runId: string
    line: string
    isError: boolean
}

export type InstallStage =
    | "prepare"
    | "download"
    | "install"
    | "finalize"
    | "finished"
    | "aborted"
    | "failed"

const TERMINAL_STAGES: InstallStage[] = ["finished", "aborted", "failed"]

export function isTerminalStage(stage: InstallStage): boolean {
    return TERMINAL_STAGES.includes(stage)
}

export interface DownloadFileProgress {
    url: string
    name: string
    loaded: number
    total: number
    percent: number
}

export interface InstallSnapshot {
    instanceId: string
    instanceName: string
    stage: InstallStage
    phase: string
    message: string
    progress: number
    files: DownloadFileProgress[]
    startedAt: number
    aborting: boolean
    error?: string
    blocked?: BlockedFile[]
    awaitingFiles: boolean
}

export type GameStatus = "starting" | "running" | "exited" | "crashed"

export interface RunningGame {
    runId: string
    instanceId: string
    instanceName: string
    pid?: number
    startedAt: number
    status: GameStatus
}
