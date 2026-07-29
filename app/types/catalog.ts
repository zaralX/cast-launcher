import type {InstanceType, PackProvider} from "~/types/instance"

export type PackSort = "relevance" | "downloads" | "follows" | "newest" | "updated"

export type PackEnvironment = "client" | "server"

export interface PackCapabilities {
    multipleGameVersions: boolean
    environment: boolean
    blockableFiles: boolean
}

export interface PackProviderInfo {
    id: PackProvider
    label: string
    ready: boolean
    reason?: string
    sorts: PackSort[]
    capabilities: PackCapabilities
}

export interface PackSearchQuery {
    provider: PackProvider
    query?: string
    categories?: string[]
    loaders?: string[]
    gameVersions?: string[]
    environment?: PackEnvironment | null
    sort?: PackSort
    offset?: number
    limit?: number
}

export interface PackHit {
    provider: PackProvider
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
    websiteUrl?: string
    distributionAllowed: boolean
}

export interface PackSearchPage {
    hits: PackHit[]
    offset: number
    limit: number
    totalHits: number
}

export interface PackFileHashes {
    sha1?: string | null
    sha512?: string | null
}

export interface PackVersionFile {
    url: string
    filename: string
    size?: number | null
    hashes: PackFileHashes
}

export interface PackVersion {
    provider: PackProvider
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
    file?: PackVersionFile | null
    blocked: boolean
    supported: boolean
}

export interface PackCategory {
    id: string
    label: string
    header: string
}

export interface PackFilters {
    categories: PackCategory[]
    loaders: string[]
    gameVersions: string[]
}

export interface BlockedFile {
    fileName: string
    targetPath: string
    websiteUrl: string
    sha1?: string
    localPath?: string
}

export const PROVIDER_LOGOS: Record<PackProvider, string> = {
    modrinth: "/modrinth.svg",
    curseforge: "/curseforge.svg"
}

export const SORT_LABELS: Record<PackSort, string> = {
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

export function categoryLabel(category: PackCategory): string {
    return CATEGORY_LABELS[category.id] ?? category.label.replace(/[-_]/g, " ")
}

export function categoryName(name: string): string {
    return CATEGORY_LABELS[name] ?? name.replace(/[-_]/g, " ")
}

export function formatDownloads(value: number): string {
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
    if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
    return String(value)
}

export function versionLabel(version: PackVersion): string {
    const parts = [version.versionNumber || version.name]

    if (version.minecraftVersion) parts.push(version.minecraftVersion)
    if (version.loaders.length) parts.push(version.loaders.map(capitalize).join("/"))

    return parts.join(" · ")
}

export function unsupportedReason(version: PackVersion): string | null {
    if (version.supported) return null
    if (version.blocked) return "автор запретил скачивание через сторонние лаунчеры"

    if (!version.loader) {
        const loaders = version.loaders.length ? version.loaders.join(", ") : "не указан"
        return `неподдерживаемый загрузчик (${loaders})`
    }

    if (!version.minecraftVersion) return "не указана версия Minecraft"

    return "в версии нет архива пака"
}

function capitalize(value: string): string {
    return value.charAt(0).toUpperCase() + value.slice(1)
}
