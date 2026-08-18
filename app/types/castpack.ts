import type {InstanceType} from "~/types/instance"

export const CASTPACK_SCHEMA_VERSION = 1

export interface CastPackSource {
    catalogId: string
    manifestUrl: string
    autoupdate: boolean
    version: string
    changelog: string
    ramApplied: boolean
}

export interface CatalogPack {
    id: string
    name: string
    summary: string
    description: string
    author: string
    version: string
    manifest: string
    icon?: string
    autoupdate: boolean
    minecraftVersion: string
    loader?: InstanceType
    tags: string[]
    website?: string
    minLauncherVersion?: string
}

export interface Catalog {
    schemaVersion: number
    updatedAt: string
    packs: CatalogPack[]
}

export interface CastPackUpdate {
    available: boolean
    version: string
    changelog: string
    error?: string
}
