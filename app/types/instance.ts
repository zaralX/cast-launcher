export type InstanceType = "vanilla" | "fabric" | "forge"

export interface Instance {
    id: string
    name: string
    description: string
    minecraftVersion: string
    type: InstanceType
    installed: boolean
    version: number
    loaderVersion?: string
    customId?: string
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
