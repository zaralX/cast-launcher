export type ModLoader = "fabric" | "quilt" | "forge" | "neoforge" | "liteloader"

export type ModKind = "jar" | "folder" | "litemod"

export interface ModDetails {
    modId: string
    name: string
    version: string
    description: string
    authors: string[]
    homepage: string
    license: string
    loaders: ModLoader[]
    iconKey?: string | null
}

export interface ModFile {
    path: string
    fileName: string
    enabled: boolean
    size: number
    modified: number
    kind: ModKind
    details: ModDetails
}

export const MOD_LOADER_LABELS: Record<ModLoader, string> = {
    fabric: "Fabric",
    quilt: "Quilt",
    forge: "Forge",
    neoforge: "NeoForge",
    liteloader: "LiteLoader"
}

export function modName(mod: ModFile): string {
    return mod.details.name.trim() || mod.fileName
}

export function modAuthors(mod: ModFile): string {
    return mod.details.authors.join(", ")
}

export function modSize(bytes: number): string {
    if (bytes <= 0) return ""
    if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} КБ`

    return `${(bytes / 1024 / 1024).toFixed(1)} МБ`
}

export function matchesMod(mod: ModFile, query: string): boolean {
    const needle = query.trim().toLowerCase()
    if (!needle) return true

    return [modName(mod), mod.details.modId, mod.fileName, modAuthors(mod)]
        .some(field => field.toLowerCase().includes(needle))
}
