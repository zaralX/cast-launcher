import type {LauncherError} from "~/types/error";

export interface Instance {
    id: string
    name: string
    description: string
    minecraftVersion: string
    type: InstanceType
    installed: boolean
    version: number
    loaderVersion?: string
    customId?: string // custom pack id, for example: modrinth slug
}

export interface LivingInstance extends Instance {
    installing: boolean
    dir: string
}

export type InstanceType = "vanilla" | "fabric" | "forge"

export interface DownloadTask {
    url: string
    destination: string
    size?: number
    verificationType?: "sha1"
    hash?: string
}

export type InstallerStage =
    | "prepare"
    | "download"
    | "install"
    | "finalize"
    | "finished"
    | "aborted"
    | "failed"

export const TERMINAL_STAGES: InstallerStage[] = ["finished", "aborted", "failed"]

export function isTerminalStage(stage: InstallerStage): boolean {
    return TERMINAL_STAGES.includes(stage)
}

export interface DownloadFileProgress {
    url: string
    name: string
    destination: string
    loaded: number
    total: number
    percent: number // 0..1
    done: boolean
}

export interface InstallPhase {
    key: string
    label: string
    weight: number
}

export interface InstallerProgress {
    stage: InstallerStage
    type?: 'global' | 'single'
    message?: string
    progress?: number
    phase?: string
    file?: DownloadFileProgress
    error?: LauncherError
}

export interface InstallProgressView {
    instanceId: string
    instanceName: string
    stage: InstallerStage
    phase: string
    message: string
    progress: number // 0..1
    files: DownloadFileProgress[]
    startedAt: number
}

export interface MojangObject {
    sha1: string
    size: number
    url: string
}

export interface MojangLibraryArtifact extends MojangObject {
    path: string
}

export interface MojangLibraryObject extends MojangLibraryArtifact {
    native?: MojangLibraryNative
}

export interface MojangLibraryNative extends MojangLibraryArtifact {

}

export interface MojangAssetIndexObject extends MojangObject {
    totalSize: number
    id: string
}

export type MinecraftStatus =
    | "starting"
    | "running"
    | "exited"
    | "error"

export interface MinecraftStatusEvent {
    status: MinecraftStatus
}

export interface MinecraftLogEvent {
    line: string
    is_error: boolean
}

export interface MinecraftEvent {
    type: 'log' | 'status' | 'exit'
    status?: MinecraftStatus
    line?: string
    code?: number | null
}