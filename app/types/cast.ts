import type { InstanceType, PackProvider } from '~/types/instance'

export const CAST_EXTENSION = '.cast'

export type FileMode = 'always' | 'once'

export interface CastExport {
  id: string
  version: string
  author: string
}

export interface CastBase {
  provider: PackProvider
  projectId: string
  versionId: string
  name: string
  version: string
}

export interface ExistingPack {
  instanceId: string
  name: string
  version: string
  minecraftVersion: string
  loader: InstanceType
}

export interface CastPreview {
  id: string
  changelog: string
  exportedBy: string
  exportedAt: number
  modrinthMods: number
  curseforgeMods: number
  linkedFiles: number
  embeddedFiles: number
  embeddedSize: number
  embeddedCode: string[]
  base?: CastBase
  hasIcon: boolean
  existing?: ExistingPack
}

export type ExportNote = 'world' | 'personal' | 'cache' | 'forbidden'

export const EXPORT_NOTE_KEYS: Record<ExportNote, string> = {
  world: 'cast_export.note.world',
  personal: 'cast_export.note.personal',
  cache: 'cast_export.note.cache',
  forbidden: 'cast_export.note.forbidden',
}

export type SourceKind = 'modrinth' | 'curseforge' | 'embedded' | 'unchecked'

export const SOURCE_KEYS: Record<SourceKind, string> = {
  modrinth: 'cast_export.source.modrinth',
  curseforge: 'cast_export.source.curseforge',
  embedded: 'cast_export.source.embedded',
  unchecked: 'cast_export.source.unchecked',
}

export interface FileSource {
  kind: SourceKind
  title?: string
}

export interface TreeEntry {
  key: string
  name: string
  dir: boolean
  size: number
  files: number
  selected: boolean
  mode: FileMode
  note?: ExportNote
  source?: FileSource
  children?: TreeEntry[]
}

export interface ExportDefaults {
  name: string
  version: string
  author: string
  description: string
  recommendedRam?: number
  knownPack: boolean
}

export interface ExportScan {
  tree: TreeEntry[]
  defaults: ExportDefaults
  unchecked: number
}

export interface ExportRequest {
  name: string
  version: string
  author: string
  description: string
  changelog: string
  recommendedRam?: number
  include: string[]
}

export interface ExportResult {
  path: string
  size: number
  mods: number
  embedded: number
  embeddedCode: number
  skipped: string[]
}

export type ExportStage = 'collecting' | 'identifying' | 'packing'

export const EXPORT_STAGE_KEYS: Record<ExportStage, string> = {
  collecting: 'cast_export.stage.collecting',
  identifying: 'cast_export.stage.identifying',
  packing: 'cast_export.stage.packing',
}

export interface ExportProgress {
  instanceId: string
  stage: ExportStage
  done: number
  total: number
}
