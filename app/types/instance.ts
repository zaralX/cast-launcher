import type { JavaMode } from '~/types/app'
import type { BlockedFile } from '~/types/catalog'
import type { CastPackSource } from '~/types/castpack'
import type { CastExport } from '~/types/cast'
import type { InstanceDir, UiText } from '~/types/backend'
import type { CommandError } from '~/types/error'

export type InstanceType = 'vanilla' | 'fabric' | 'forge' | 'neoforge'

export const INSTANCE_TYPE_LABELS: Record<InstanceType, string> = {
  vanilla: 'Vanilla',
  fabric: 'Fabric',
  forge: 'Forge',
  neoforge: 'NeoForge',
}

export interface InstanceSettings {
  overrideMemory: boolean
  minRam: number
  maxRam: number
  overrideJava: boolean
  javaMode: JavaMode
  javaPath: string
}

export type PackProvider = 'modrinth' | 'curseforge'

export const PACK_PROVIDER_LABELS: Record<PackProvider, string> = {
  modrinth: 'Modrinth',
  curseforge: 'CurseForge',
}

export interface PackSource {
  provider: PackProvider
  projectId: string
  versionId: string
  versionNumber: string
  fileUrl: string
  fileName: string
  fileSha1?: string
  fileSize?: number
}

export type LocalPackKind = 'modrinth' | 'curseforge' | 'multimc' | 'cast'

export const LOCAL_PACK_KIND_LABELS: Record<LocalPackKind, string> = {
  modrinth: 'Modrinth (.mrpack)',
  curseforge: 'CurseForge',
  multimc: 'MultiMC / Prism',
  cast: 'Cast Launcher (.cast)',
}

export interface LocalPackSource {
  kind: LocalPackKind
  name: string
  version: string
}

export interface Playtime {
  totalSeconds: number
  lastSeconds: number
  lastPlayedAt: number
}

export interface Instance {
  id: string
  name: string
  description: string
  minecraftVersion: string
  icon: string
  type: InstanceType
  installed: boolean
  version: number
  loaderVersion?: string
  customId?: string
  pack?: PackSource
  castpack?: CastPackSource
  localPack?: LocalPackSource
  castExport?: CastExport
  settings: InstanceSettings
  playtime: Playtime
}

export interface InstanceLogFile {
  name: string
  size: number
  modified: number
}

export interface GameLogLine {
  runId: string
  line: string
  isError: boolean
}

export type InstallStage
  = | 'prepare'
    | 'download'
    | 'install'
    | 'finalize'
    | 'finished'
    | 'aborted'
    | 'failed'

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
  phase: UiText
  message: UiText
  progress: number
  files: DownloadFileProgress[]
  startedAt: number
  aborting: boolean
  error?: CommandError
  blocked?: BlockedFile[]
  awaitingFiles: boolean
}

export type GameStatus = 'starting' | 'running' | 'exited' | 'crashed'

export interface RunningGame {
  runId: string
  instanceId: string
  instanceName: string
  pid?: number
  startedAt: number
  status: GameStatus
}

export type InstanceState = 'running' | 'installing' | 'ready' | 'absent'

export const INSTANCE_DIRS = ['root', 'minecraft', 'logs'] as const satisfies readonly InstanceDir[]

export type InstanceDirTarget = typeof INSTANCE_DIRS[number]

export const INSTANCE_DIR_KEYS: Record<InstanceDirTarget, string> = {
  root: 'instance.dir.root',
  minecraft: 'instance.dir.minecraft',
  logs: 'instance.dir.logs',
}
