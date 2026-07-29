import type {JavaMode} from "~/types/app"

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

export type PackProvider = "modrinth"

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
    settings: InstanceSettings
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
