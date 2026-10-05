import type { Account, AccountConfig } from '~/types/account'
import type { AppConfig, JavaRuntime, LauncherPaths } from '~/types/app'
import type { Instance, InstanceLogFile, InstanceSettings, InstallSnapshot, PackSource, RunningGame, PackProvider } from '~/types/instance'
import type { IconFile, ItemCatalog } from '~/types/icon'
import type { CatalogMatch, CatalogVersion, InstallPlan, InstallReport, InstalledMods, ModFile, ModSearchQuery, ModUpdate, UpdatedModsReport } from '~/types/mods'
import type { AccountLook, SkinEntry, SkinLibrary, SkinVariant } from '~/types/skin'
import type { DetectedLauncher, FileImportRequest, ImportProgress, ImportReport, ImportRequest, LauncherKind, LocalPack, ScannedInstance } from '~/types/import'
import type { Catalog, CastPackUpdate } from '~/types/castpack'
import type { BlockedFile, PackFilters, PackProviderInfo, PackSearchPage, PackSearchQuery, PackVersion } from '~/types/catalog'

export const LAUNCHER_EVENT = 'launcher://event'

export type LauncherEvent
  = | (InstallSnapshot & { type: 'install' })
    | (ImportProgress & { type: 'import' })
    | { type: 'importFinished', report: ImportReport }
    | { type: 'instances', instances: Instance[] }
    | { type: 'gameStarted', game: RunningGame }
    | { type: 'gameStatus', runId: string, instanceId: string, status: RunningGame['status'] }
    | { type: 'gameLog', runId: string, instanceId: string, line: string, isError: boolean }
    | { type: 'gameExited', runId: string, instanceId: string, code: number | null, logTail?: string }
    | { type: 'launchFailed', instanceId: string, instanceName: string, error: string }

export interface Bootstrap {
  config: AppConfig
  paths: LauncherPaths
  accounts: AccountConfig
  instances: Instance[]
  installs: InstallSnapshot[]
  running: RunningGame[]
  import?: ImportProgress | null
}

export interface Commands {
  bootstrap: [undefined, Bootstrap]

  get_config: [undefined, AppConfig]
  update_config: [{ config: AppConfig }, AppConfig]
  get_paths: [undefined, LauncherPaths]
  open_path: [{ path: string }, undefined]
  open_url: [{ url: string }, undefined]

  list_instances: [undefined, Instance[]]
  reload_instances: [undefined, Instance[]]
  create_instance: [{ instance: NewInstance }, Instance]
  update_instance: [{ instanceId: string, update: InstanceUpdate }, Instance]
  delete_instance: [{ instanceId: string }, undefined]
  open_instance_dir: [{ instanceId: string, target: InstanceDir }, undefined]

  list_instance_logs: [{ instanceId: string }, InstanceLogFile[]]
  read_instance_log: [{ instanceId: string, name: string }, string]
  delete_instance_log: [{ instanceId: string, name: string }, InstanceLogFile[]]

  list_instance_mods: [{ instanceId: string }, ModFile[]]
  refresh_instance_mods: [{ instanceId: string }, ModFile[]]
  read_mod_icon: [{ key: string }, string]
  set_mod_enabled: [{ instanceId: string, path: string, enabled: boolean }, ModFile[]]
  delete_mods: [{ instanceId: string, paths: string[] }, ModFile[]]
  add_mods: [{ instanceId: string, paths: string[] }, AddedMods]
  pick_mod_files: [undefined, string[]]
  identify_instance_mods: [{ instanceId: string }, Record<string, CatalogMatch>]
  check_mod_updates: [{ instanceId: string }, ModUpdate[]]
  update_mods: [{ instanceId: string, paths: string[] }, UpdatedMods]
  search_mods: [{ instanceId: string, query: ModSearchQuery }, PackSearchPage]
  mod_versions: [{ instanceId: string, provider: PackProvider, projectId: string }, CatalogVersion[]]
  plan_mod_install: [
    { instanceId: string, provider: PackProvider, projectId: string, versionId: string },
    InstallPlan,
  ]
  install_mod: [{ instanceId: string, planId: string, optional: string[] }, InstalledFromCatalog]

  list_icons: [undefined, IconFile[]]
  read_icon: [{ name: string }, string]
  import_icon: [{ path?: string }, IconFile | null]
  delete_icon: [{ name: string }, IconFile[]]
  list_item_icons: [undefined, ItemCatalog]
  item_icons: [{ items: string[] }, Record<string, string>]
  save_item_icon: [{ item: string }, IconFile]

  install_instance: [{ instanceId: string }, InstallSnapshot]
  cancel_install: [{ instanceId: string }, undefined]
  list_installs: [undefined, InstallSnapshot[]]

  awaited_files: [{ instanceId: string }, BlockedFile[]]
  downloads_dir: [undefined, string | null]
  scan_for_files: [{ instanceId: string, folder: string }, BlockedFile[]]
  rescan_files: [{ instanceId: string }, BlockedFile[]]
  pick_folder: [{ title?: string, directory?: string }, string | null]
  resume_install: [{ instanceId: string }, undefined]

  launch_instance: [{ instanceId: string }, RunningGame]
  play_instance: [{ instanceId: string }, PlayOutcome]
  list_running: [undefined, RunningGame[]]
  stop_instance: [{ instanceId: string }, number]

  list_java: [{ force: boolean }, JavaRuntime[]]
  probe_java: [{ path: string }, JavaRuntime | null]

  list_accounts: [undefined, AccountConfig]
  select_account: [{ index: number }, AccountConfig]
  remove_account: [{ uuid: string }, AccountConfig]
  add_offline_account: [{ name: string }, AccountConfig]
  login_microsoft: [undefined, Account]
  refresh_account: [{ uuid: string }, Account]

  skin_library: [undefined, SkinLibrary]
  skin_texture: [{ texture: string }, string]
  import_skin: [{ path?: string }, SkinEntry | null]
  import_player_skin: [{ name: string }, SkinEntry]
  rename_skin: [{ id: string, name: string }, SkinLibrary]
  set_skin_variant: [{ id: string, variant: SkinVariant }, SkinLibrary]
  set_skin_cape: [{ id: string, capeId?: string | null }, SkinLibrary]
  duplicate_skin: [{ id: string, capeId?: string | null }, SkinEntry]
  delete_skin: [{ id: string }, SkinLibrary]
  account_look: [{ uuid: string, refresh: boolean }, AccountLook]
  apply_skin: [{ uuid: string, id: string }, AccountLook]
  reset_skin: [{ uuid: string }, AccountLook]
  apply_cape: [{ uuid: string, capeId?: string | null }, AccountLook]

  castpack_catalog: [undefined, Catalog]
  castpack_install: [{ packId: string }, Instance]
  castpack_check_update: [{ instanceId: string }, CastPackUpdate]
  castpack_set_autoupdate: [{ instanceId: string, enabled: boolean }, Instance]

  pack_providers: [undefined, PackProviderInfo[]]
  search_packs: [{ query: PackSearchQuery }, PackSearchPage]
  list_pack_versions: [{ provider: PackProvider, projectId: string }, PackVersion[]]
  pack_filters: [{ provider: PackProvider }, PackFilters]
  set_instance_pack_version: [{ instanceId: string, versionId: string }, Instance]
  list_pack_blocked: [{ instanceId: string }, BlockedFile[]]
  save_pack_icon: [{ provider: PackProvider, projectId: string, url: string }, IconFile]

  detect_launchers: [undefined, DetectedLauncher[]]
  pick_launcher_dir: [undefined, string | null]
  scan_launcher_instances: [{ kind: LauncherKind, path: string }, ScannedInstance[]]
  import_launcher_instances: [{ request: ImportRequest }, ImportReport]
  cancel_import: [undefined, undefined]

  pick_modpack_file: [undefined, string | null]
  inspect_modpack_file: [{ path: string }, LocalPack]
  import_modpack_file: [{ request: FileImportRequest }, Instance]

  list_minecraft_versions: [undefined, VersionManifest]
  list_fabric_versions: [undefined, string[]]
  list_forge_versions: [undefined, string[]]
  list_neoforge_versions: [undefined, NeoForgeRelease[]]
}

export interface AddedMods {
  mods: ModFile[]
  report: InstalledMods
}

export interface UpdatedMods {
  mods: ModFile[]
  report: UpdatedModsReport
}

export interface InstalledFromCatalog {
  mods: ModFile[]
  report: InstallReport
}

export type PlayOutcome
  = | { kind: 'launched', game: RunningGame }
    | { kind: 'installing', install: InstallSnapshot }

export interface NeoForgeRelease {
  version: string
  minecraftVersion: string
}

export interface NewInstance {
  id: string
  name: string
  description: string
  minecraftVersion: string
  type: Instance['type']
  version: number
  icon?: string
  loaderVersion?: string
  customId?: string
  pack?: PackSource
  settings?: InstanceSettings
}

export interface InstanceUpdate {
  name?: string
  description?: string
  icon?: string
  settings?: InstanceSettings
}

export type InstanceDir = 'root' | 'minecraft' | 'mods' | 'logs'

export interface VersionManifest {
  latest: { release?: string, snapshot?: string }
  versions: { id: string, url: string, type?: string, sha1?: string }[]
}
