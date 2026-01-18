export interface Instance {
    id: string
    name: string
    description: string
    minecraftVersion: string
    type: InstanceType
    installed: boolean
    version: number
    loaderVersion?: string
}

export interface LivingInstance extends Instance {
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

export interface InstallerProgress {
    stage: InstallerStage
    type?: 'global' | 'single'
    message?: string
    progress?: number // 0..1
}

export interface MojangObject {
    sha1: string
    size: number
    url: string
}

export interface MojangLibraryObject extends MojangObject {
    path: string
}