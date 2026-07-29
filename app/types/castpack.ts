import type {InstanceType, PackProvider} from "~/types/instance"

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

export type CastPackFileMode = "always" | "once"

export interface CastPackMod {
    provider?: PackProvider
    projectId: string
    versionId: string
    url: string
    path: string
    sha1?: string
    size?: number
    optional: boolean
}

export interface CastPackFile {
    path: string
    url: string
    sha1?: string
    size?: number
    mode: CastPackFileMode
}

export interface CastPackBase {
    provider: PackProvider
    projectId: string
    versionId: string
}

export interface CastPackLoader {
    type: InstanceType
    version?: string
}

export interface CastPackManifest {
    schemaVersion: number
    id: string
    name: string
    version: string
    changelog: string
    minecraft: string
    loader?: CastPackLoader
    base?: CastPackBase
    mods: CastPackMod[]
    files: CastPackFile[]
    delete: string[]
    settings: { recommendedRam?: number }
}

export interface ProbedFile {
    fileName: string
    sha1: string
    size: number
}

export interface ProbedMod {
    path: string
    url: string
    sha1?: string
    size?: number
    blocked: boolean
}

export function emptyManifest(): CastPackManifest {
    return {
        schemaVersion: CASTPACK_SCHEMA_VERSION,
        id: "",
        name: "",
        version: "1.0.0",
        changelog: "",
        minecraft: "",
        mods: [],
        files: [],
        delete: [],
        settings: {}
    }
}

export function emptyMod(): CastPackMod {
    return {
        provider: "modrinth",
        projectId: "",
        versionId: "",
        url: "",
        path: "",
        optional: false
    }
}

export function emptyFile(): CastPackFile {
    return {path: "", url: "", mode: "always"}
}

export function cleanManifest(manifest: CastPackManifest): Record<string, unknown> {
    const clean: Record<string, unknown> = {
        schemaVersion: CASTPACK_SCHEMA_VERSION,
        id: manifest.id.trim(),
        name: manifest.name.trim(),
        version: manifest.version.trim()
    }

    if (manifest.changelog.trim()) clean.changelog = manifest.changelog.trim()
    if (manifest.minecraft.trim()) clean.minecraft = manifest.minecraft.trim()

    if (manifest.loader) {
        const loader: Record<string, unknown> = {type: manifest.loader.type}
        if (manifest.loader.version?.trim()) loader.version = manifest.loader.version.trim()
        clean.loader = loader
    }

    if (manifest.base) {
        clean.base = {
            provider: manifest.base.provider,
            projectId: manifest.base.projectId.trim(),
            versionId: manifest.base.versionId.trim()
        }
    }

    if (manifest.mods.length) clean.mods = manifest.mods.map(cleanMod)
    if (manifest.files.length) clean.files = manifest.files.map(cleanFile)

    const removed = manifest.delete.map(path => path.trim()).filter(Boolean)
    if (removed.length) clean.delete = removed

    if (manifest.settings.recommendedRam) {
        clean.settings = {recommendedRam: manifest.settings.recommendedRam}
    }

    return clean
}

function cleanMod(mod: CastPackMod): Record<string, unknown> {
    const clean: Record<string, unknown> = {}

    if (mod.url.trim()) {
        clean.url = mod.url.trim()
        clean.path = mod.path.trim()
        if (mod.sha1?.trim()) clean.sha1 = mod.sha1.trim()
        if (mod.size) clean.size = mod.size
    } else {
        clean.provider = mod.provider
        clean.projectId = mod.projectId.trim()
        clean.versionId = mod.versionId.trim()
    }

    if (mod.optional) clean.optional = true

    return clean
}

function cleanFile(file: CastPackFile): Record<string, unknown> {
    const clean: Record<string, unknown> = {
        path: file.path.trim(),
        url: file.url.trim(),
        mode: file.mode
    }

    if (file.sha1?.trim()) clean.sha1 = file.sha1.trim()
    if (file.size) clean.size = file.size

    return clean
}

export function manifestJson(manifest: CastPackManifest): string {
    return JSON.stringify(cleanManifest(manifest), null, 2)
}
