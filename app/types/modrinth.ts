import type {InstanceType} from "~/types/instance"

export type ModrinthSort = "relevance" | "downloads" | "follows" | "newest" | "updated"

export type PackEnvironment = "client" | "server"

export interface ModrinthSearchQuery {
    query?: string
    categories?: string[]
    loaders?: string[]
    gameVersions?: string[]
    environment?: PackEnvironment | null
    sort?: ModrinthSort
    offset?: number
    limit?: number
}

export interface ModrinthHit {
    projectId: string
    slug: string
    title: string
    description: string
    iconUrl?: string | null
    author?: string | null
    downloads: number
    follows: number
    categories: string[]
    displayCategories: string[]
    versions: string[]
    clientSide?: string | null
    serverSide?: string | null
    dateModified?: string | null
}

export interface ModrinthSearchPage {
    hits: ModrinthHit[]
    offset: number
    limit: number
    totalHits: number
}

export interface ModrinthFileHashes {
    sha1?: string | null
    sha512?: string | null
}

export interface ModrinthVersionFile {
    url: string
    filename: string
    size?: number | null
    primary: boolean
    hashes: ModrinthFileHashes
}

export interface ModrinthVersion {
    id: string
    projectId: string
    name: string
    versionNumber: string
    versionType: string
    downloads: number
    datePublished?: string | null
    gameVersions: string[]
    loaders: string[]
    minecraftVersion?: string | null
    loader?: InstanceType | null
    file?: ModrinthVersionFile | null
    supported: boolean
}

export interface ModrinthCategory {
    name: string
    header: string
}

export interface ModrinthFilters {
    categories: ModrinthCategory[]
    loaders: string[]
    gameVersions: string[]
}

export const SORT_LABELS: Record<ModrinthSort, string> = {
    relevance: "По совпадению",
    downloads: "По загрузкам",
    follows: "По подпискам",
    newest: "Новые",
    updated: "Обновлённые"
}

const CATEGORY_LABELS: Record<string, string> = {
    adventure: "Приключения",
    challenging: "Хардкор",
    combat: "Сражения",
    "kitchen-sink": "Всё сразу",
    lightweight: "Лёгкие",
    magic: "Магия",
    multiplayer: "Мультиплеер",
    optimization: "Оптимизация",
    quests: "Квесты",
    technology: "Технологии",
    fabric: "Fabric",
    forge: "Forge",
    neoforge: "NeoForge",
    quilt: "Quilt"
}

export function categoryLabel(name: string): string {
    return CATEGORY_LABELS[name] ?? name.replace(/[-_]/g, " ")
}

export function formatDownloads(value: number): string {
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
    if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
    return String(value)
}

export function versionLabel(version: ModrinthVersion): string {
    const parts = [version.versionNumber || version.name]

    if (version.minecraftVersion) parts.push(version.minecraftVersion)
    if (version.loaders.length) parts.push(version.loaders.map(capitalize).join("/"))

    return parts.join(" · ")
}

function capitalize(value: string): string {
    return value.charAt(0).toUpperCase() + value.slice(1)
}
