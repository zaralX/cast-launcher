export type ModLoader = 'fabric' | 'quilt' | 'forge' | 'neoforge' | 'liteloader'

export type ModKind = 'jar' | 'folder' | 'litemod'

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
  managed: boolean
  details: ModDetails
}

export interface InstalledMods {
  added: string[]
  replaced: string[]
  skipped: string[]
  failed: string[]
}

export interface CatalogMatch {
  provider: 'modrinth' | 'curseforge'
  projectId: string
  versionId: string
  versionNumber: string
  title: string
  slug: string
  iconUrl: string
  pageUrl: string
  authors: string[]
}

export interface ModUpdate {
  path: string
  provider: 'modrinth' | 'curseforge'
  projectId: string
  title: string
  from: string
  to: string
  versionId: string
  fileName: string
  url: string
  sha1?: string
  size?: number
  pageUrl: string
}

export interface UpdatedModsReport {
  updated: string[]
  failed: string[]
}

export const CATALOG_LABELS: Record<CatalogMatch['provider'], string> = {
  modrinth: 'Modrinth',
  curseforge: 'CurseForge',
}

export interface CatalogVersion {
  versionId: string
  versionNumber: string
  fileName: string
  url: string
  sha1?: string
  size?: number
  date?: string
  release: string
  blocked: boolean
  dependencies: {
    projectId: string
    versionId?: string
    required: boolean
  }[]
}

export interface ModSearchQuery {
  provider: CatalogMatch['provider']
  query?: string
  sort?: string
  offset?: number
  limit?: number
}

export interface PlannedMod {
  provider: CatalogMatch['provider']
  projectId: string
  versionId: string
  versionNumber: string
  title: string
  fileName: string
  url: string
  sha1?: string
  size?: number
  iconUrl: string
  pageUrl: string
  blocked: boolean
}

export interface InstallPlan {
  id: string
  target: PlannedMod
  required: PlannedMod[]
  optional: PlannedMod[]
  installed: string[]
}

export interface InstallReport {
  installed: string[]
  failed: string[]
  blocked: string[]
}

export const RELEASE_KEYS: Record<string, string> = {
  release: 'catalog.release.release',
  beta: 'catalog.release.beta',
  alpha: 'catalog.release.alpha',
}

export const MOD_EXTENSIONS = ['jar', 'zip', 'litemod']

export const MOD_LOADER_LABELS: Record<ModLoader, string> = {
  fabric: 'Fabric',
  quilt: 'Quilt',
  forge: 'Forge',
  neoforge: 'NeoForge',
  liteloader: 'LiteLoader',
}
