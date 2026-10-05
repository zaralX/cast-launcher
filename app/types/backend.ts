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

type Command<Args, Result> = [args: Args, result: Result]

export interface Commands {
  bootstrap: Command<void, Bootstrap>

  get_config: Command<void, AppConfig>
  update_config: Command<{ config: AppConfig }, AppConfig>
  get_paths: Command<void, LauncherPaths>
  open_path: Command<{ path: string }, void>
  open_url: Command<{ url: string }, void>

  list_instances: Command<void, Instance[]>
  reload_instances: Command<void, Instance[]>
  create_instance: Command<{ instance: NewInstance }, Instance>
  update_instance: Command<{ instanceId: string, update: InstanceUpdate }, Instance>
  delete_instance: Command<{ instanceId: string }, void>
  open_instance_dir: Command<{ instanceId: string, target: InstanceDir }, void>

  list_instance_logs: Command<{ instanceId: string }, InstanceLogFile[]>
  read_instance_log: Command<{ instanceId: string, name: string }, string>
  delete_instance_log: Command<{ instanceId: string, name: string }, InstanceLogFile[]>

  list_instance_mods: Command<{ instanceId: string }, ModFile[]>
  refresh_instance_mods: Command<{ instanceId: string }, ModFile[]>
  read_mod_icon: Command<{ key: string }, string>
  set_mod_enabled: Command<{ instanceId: string, path: string, enabled: boolean }, ModFile[]>
  delete_mods: Command<{ instanceId: string, paths: string[] }, ModFile[]>
  add_mods: Command<{ instanceId: string, paths: string[] }, AddedMods>
  pick_mod_files: Command<void, string[]>
  identify_instance_mods: Command<{ instanceId: string }, Record<string, CatalogMatch>>
  check_mod_updates: Command<{ instanceId: string }, ModUpdate[]>
  update_mods: Command<{ instanceId: string, paths: string[] }, UpdatedMods>
  search_mods: Command<{ instanceId: string, query: ModSearchQuery }, PackSearchPage>
  mod_versions: Command<{ instanceId: string, provider: PackProvider, projectId: string }, CatalogVersion[]>
  plan_mod_install: Command<
    { instanceId: string, provider: PackProvider, projectId: string, versionId: string },
    InstallPlan
  >
  install_mod: Command<{ instanceId: string, planId: string, optional: string[] }, InstalledFromCatalog>

  list_icons: Command<void, IconFile[]>
  read_icon: Command<{ name: string }, string>
  import_icon: Command<{ path?: string }, IconFile | null>
  delete_icon: Command<{ name: string }, IconFile[]>
  list_item_icons: Command<void, ItemCatalog>
  item_icons: Command<{ items: string[] }, Record<string, string>>
  save_item_icon: Command<{ item: string }, IconFile>

  install_instance: Command<{ instanceId: string }, InstallSnapshot>
  cancel_install: Command<{ instanceId: string }, void>
  list_installs: Command<void, InstallSnapshot[]>

  awaited_files: Command<{ instanceId: string }, BlockedFile[]>
  downloads_dir: Command<void, string | null>
  scan_for_files: Command<{ instanceId: string, folder: string }, BlockedFile[]>
  rescan_files: Command<{ instanceId: string }, BlockedFile[]>
  pick_folder: Command<{ title?: string, directory?: string }, string | null>
  resume_install: Command<{ instanceId: string }, void>

  launch_instance: Command<{ instanceId: string }, RunningGame>
  play_instance: Command<{ instanceId: string }, PlayOutcome>
  list_running: Command<void, RunningGame[]>
  stop_instance: Command<{ instanceId: string }, number>

  list_java: Command<{ force: boolean }, JavaRuntime[]>
  probe_java: Command<{ path: string }, JavaRuntime | null>

  list_accounts: Command<void, AccountConfig>
  select_account: Command<{ index: number }, AccountConfig>
  remove_account: Command<{ uuid: string }, AccountConfig>
  add_offline_account: Command<{ name: string }, AccountConfig>
  login_microsoft: Command<void, Account>
  refresh_account: Command<{ uuid: string }, Account>

  skin_library: Command<void, SkinLibrary>
  skin_texture: Command<{ texture: string }, string>
  import_skin: Command<{ path?: string }, SkinEntry | null>
  import_player_skin: Command<{ name: string }, SkinEntry>
  rename_skin: Command<{ id: string, name: string }, SkinLibrary>
  set_skin_variant: Command<{ id: string, variant: SkinVariant }, SkinLibrary>
  set_skin_cape: Command<{ id: string, capeId?: string | null }, SkinLibrary>
  duplicate_skin: Command<{ id: string, capeId?: string | null }, SkinEntry>
  delete_skin: Command<{ id: string }, SkinLibrary>
  account_look: Command<{ uuid: string, refresh: boolean }, AccountLook>
  apply_skin: Command<{ uuid: string, id: string }, AccountLook>
  reset_skin: Command<{ uuid: string }, AccountLook>
  apply_cape: Command<{ uuid: string, capeId?: string | null }, AccountLook>

  castpack_catalog: Command<void, Catalog>
  castpack_install: Command<{ packId: string }, Instance>
  castpack_check_update: Command<{ instanceId: string }, CastPackUpdate>
  castpack_set_autoupdate: Command<{ instanceId: string, enabled: boolean }, Instance>

  pack_providers: Command<void, PackProviderInfo[]>
  search_packs: Command<{ query: PackSearchQuery }, PackSearchPage>
  list_pack_versions: Command<{ provider: PackProvider, projectId: string }, PackVersion[]>
  pack_filters: Command<{ provider: PackProvider }, PackFilters>
  set_instance_pack_version: Command<{ instanceId: string, versionId: string }, Instance>
  list_pack_blocked: Command<{ instanceId: string }, BlockedFile[]>
  save_pack_icon: Command<{ provider: PackProvider, projectId: string, url: string }, IconFile>

  detect_launchers: Command<void, DetectedLauncher[]>
  pick_launcher_dir: Command<void, string | null>
  scan_launcher_instances: Command<{ kind: LauncherKind, path: string }, ScannedInstance[]>
  import_launcher_instances: Command<{ request: ImportRequest }, ImportReport>
  cancel_import: Command<void, void>

  pick_modpack_file: Command<void, string | null>
  inspect_modpack_file: Command<{ path: string }, LocalPack>
  import_modpack_file: Command<{ request: FileImportRequest }, Instance>

  list_minecraft_versions: Command<void, VersionManifest>
  list_fabric_versions: Command<void, string[]>
  list_forge_versions: Command<void, string[]>
  list_neoforge_versions: Command<void, NeoForgeRelease[]>
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
