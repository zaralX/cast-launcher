import type { InstanceSettings, InstanceType, LocalPackKind, Playtime } from '~/types/instance'

export type LauncherKind = 'prism' | 'modrinth'

export interface DetectedLauncher {
  kind: LauncherKind
  label: string
  path: string
  instances: number
}

export interface ScannedPack {
  provider: string
  projectId: string
  versionId: string
  versionName: string
  name: string
}

export interface ScannedInstance {
  folder: string
  name: string
  description: string
  minecraftVersion: string
  loader?: InstanceType
  loaderVersion?: string
  loaderLabel: string
  icon?: string
  settings: InstanceSettings
  playtime: Playtime
  pack?: ScannedPack
  blocked?: string
}

export interface LocalPack {
  kind: LocalPackKind
  kindLabel: string
  path: string
  fileName: string
  size: number
  name: string
  version: string
  author: string
  description: string
  minecraftVersion: string
  loader?: InstanceType
  loaderVersion?: string
  loaderLabel: string
  files: number
  settings: InstanceSettings
  blocked?: string
}

export interface FileImportRequest {
  path: string
  name?: string
  description?: string
}

export interface ImportOptions {
  assets: boolean
  libraries: boolean
  java: boolean
  icons: boolean
  linkPacks: boolean
}

export interface ImportRequest {
  kind: LauncherKind
  path: string
  folders: string[]
  options: ImportOptions
}

export type ImportStage = 'shared' | 'instances' | 'done'

export interface CopyStats {
  files: number
  bytes: number
  skipped: number
}

export interface ImportProgress {
  source: LauncherKind
  stage: ImportStage
  step: string
  done: number
  total: number
  stats: CopyStats
}

export interface ImportedInstance {
  id: string
  name: string
  linked: boolean
}

export interface SkippedInstance {
  name: string
  reason: string
}

export interface ImportReport {
  imported: ImportedInstance[]
  skipped: SkippedInstance[]
  stats: CopyStats
  cancelled: boolean
}
