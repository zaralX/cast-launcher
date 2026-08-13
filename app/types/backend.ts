import {invoke} from "@tauri-apps/api/core"
import {listen} from "@tauri-apps/api/event"
import type {Account, AccountConfig} from "~/types/account"
import type {AppConfig, JavaRuntime, LauncherPaths} from "~/types/app"
import type {
    Instance,
    InstanceLogFile,
    InstanceSettings,
    InstallSnapshot,
    PackSource,
    RunningGame
} from "~/types/instance"
import type {IconFile, ItemCatalog} from "~/types/icon"
import type {
    DetectedLauncher,
    FileImportRequest,
    ImportProgress,
    ImportReport,
    ImportRequest,
    LauncherKind,
    LocalPack,
    ScannedInstance
} from "~/types/import"
import type {PackProvider} from "~/types/instance"
import type {
    Catalog,
    CastPackManifest,
    CastPackUpdate,
    ProbedFile,
    ProbedMod
} from "~/types/castpack"
import type {
    BlockedFile,
    PackFilters,
    PackProviderInfo,
    PackSearchPage,
    PackSearchQuery,
    PackVersion
} from "~/types/catalog"
import {toLauncherError} from "~/types/error"

const LAUNCHER_EVENT = "launcher://event"

export type LauncherEvent =
    | (InstallSnapshot & { type: "install" })
    | (ImportProgress & { type: "import" })
    | { type: "instances", instances: Instance[] }
    | { type: "gameStarted", game: RunningGame }
    | { type: "gameStatus", runId: string, instanceId: string, status: RunningGame["status"] }
    | { type: "gameLog", runId: string, instanceId: string, line: string, isError: boolean }
    | { type: "gameExited", runId: string, instanceId: string, code: number | null, logTail?: string }
    | { type: "launchFailed", instanceId: string, instanceName: string, error: string }

export interface Bootstrap {
    config: AppConfig
    paths: LauncherPaths
    accounts: AccountConfig
    instances: Instance[]
    installs: InstallSnapshot[]
    running: RunningGame[]
}

interface Commands {
    bootstrap: [void, Bootstrap]

    get_config: [void, AppConfig]
    update_config: [{ config: AppConfig }, AppConfig]
    get_paths: [void, LauncherPaths]
    open_path: [{ path: string }, void]
    open_url: [{ url: string }, void]

    list_instances: [void, Instance[]]
    reload_instances: [void, Instance[]]
    create_instance: [{ instance: NewInstance }, Instance]
    update_instance: [{ instanceId: string, update: InstanceUpdate }, Instance]
    delete_instance: [{ instanceId: string }, void]
    open_instance_dir: [{ instanceId: string, target: InstanceDir }, void]

    list_instance_logs: [{ instanceId: string }, InstanceLogFile[]]
    read_instance_log: [{ instanceId: string, name: string }, string]
    delete_instance_log: [{ instanceId: string, name: string }, InstanceLogFile[]]

    list_icons: [void, IconFile[]]
    read_icon: [{ name: string }, string]
    import_icon: [{ path?: string }, IconFile | null]
    delete_icon: [{ name: string }, IconFile[]]
    list_item_icons: [void, ItemCatalog]
    item_icons: [{ items: string[] }, Record<string, string>]
    save_item_icon: [{ item: string }, IconFile]

    install_instance: [{ instanceId: string }, InstallSnapshot]
    cancel_install: [{ instanceId: string }, void]
    list_installs: [void, InstallSnapshot[]]

    awaited_files: [{ instanceId: string }, BlockedFile[]]
    downloads_dir: [void, string | null]
    scan_for_files: [{ instanceId: string, folder: string }, BlockedFile[]]
    rescan_files: [{ instanceId: string }, BlockedFile[]]
    pick_folder: [{ title?: string, directory?: string }, string | null]
    resume_install: [{ instanceId: string }, void]

    launch_instance: [{ instanceId: string }, RunningGame]
    play_instance: [{ instanceId: string }, PlayOutcome]
    list_running: [void, RunningGame[]]
    stop_instance: [{ instanceId: string }, number]

    list_java: [{ force: boolean }, JavaRuntime[]]
    probe_java: [{ path: string }, JavaRuntime | null]

    list_accounts: [void, AccountConfig]
    select_account: [{ index: number }, AccountConfig]
    remove_account: [{ uuid: string }, AccountConfig]
    add_offline_account: [{ name: string }, AccountConfig]
    login_microsoft: [void, Account]
    refresh_account: [{ uuid: string }, Account]

    castpack_catalog: [void, Catalog]
    castpack_install: [{ packId: string }, Instance]
    castpack_check_update: [{ instanceId: string }, CastPackUpdate]
    castpack_set_autoupdate: [{ instanceId: string, enabled: boolean }, Instance]
    castpack_validate: [{ json: string }, CastPackManifest]
    castpack_save_manifest: [{ json: string }, string | null]
    castpack_probe_file: [{ url: string }, ProbedFile]
    castpack_probe_mod: [{ provider: PackProvider, projectId: string, versionId: string }, ProbedMod]

    pack_providers: [void, PackProviderInfo[]]
    search_packs: [{ query: PackSearchQuery }, PackSearchPage]
    list_pack_versions: [{ provider: PackProvider, projectId: string }, PackVersion[]]
    pack_filters: [{ provider: PackProvider }, PackFilters]
    set_instance_pack_version: [{ instanceId: string, versionId: string }, Instance]
    list_pack_blocked: [{ instanceId: string }, BlockedFile[]]
    save_pack_icon: [{ provider: PackProvider, projectId: string, url: string }, IconFile]

    detect_launchers: [void, DetectedLauncher[]]
    pick_launcher_dir: [void, string | null]
    scan_launcher_instances: [{ kind: LauncherKind, path: string }, ScannedInstance[]]
    import_launcher_instances: [{ request: ImportRequest }, ImportReport]
    cancel_import: [void, void]

    pick_modpack_file: [void, string | null]
    inspect_modpack_file: [{ path: string }, LocalPack]
    import_modpack_file: [{ request: FileImportRequest }, Instance]

    list_minecraft_versions: [void, VersionManifest]
    list_fabric_versions: [void, string[]]
    list_forge_versions: [void, string[]]
    list_neoforge_versions: [void, NeoForgeRelease[]]
}

export type PlayOutcome =
    | { kind: "launched", game: RunningGame }
    | { kind: "installing", install: InstallSnapshot }

export interface NeoForgeRelease {
    version: string
    minecraftVersion: string
}

export interface NewInstance {
    id: string
    name: string
    description: string
    minecraftVersion: string
    type: Instance["type"]
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

export type InstanceDir = "root" | "minecraft" | "logs"

export interface VersionManifest {
    latest: { release?: string, snapshot?: string }
    versions: { id: string, url: string, type?: string, sha1?: string }[]
}

type Args<K extends keyof Commands> = Commands[K][0]
type Result<K extends keyof Commands> = Commands[K][1]

export async function call<K extends keyof Commands>(
    ...[command, args]: Args<K> extends void ? [K] : [K, Args<K>]
): Promise<Result<K>> {
    try {
        return await invoke<Result<K>>(command, args as Record<string, unknown> | undefined)
    } catch (e) {
        throw toLauncherError(e, "UNKNOWN", {command})
    }
}

export async function onLauncherEvent(handler: (event: LauncherEvent) => void) {
    return await listen<LauncherEvent>(LAUNCHER_EVENT, e => handler(e.payload))
}
