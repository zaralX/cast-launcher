import {invoke} from "@tauri-apps/api/core"
import {listen} from "@tauri-apps/api/event"
import type {Account, AccountConfig} from "~/types/account"
import type {AppConfig, JavaRuntime, LauncherPaths} from "~/types/app"
import type {Instance, InstallSnapshot, RunningGame} from "~/types/instance"
import type {MyPacksConfig} from "~/types/pack"
import {toLauncherError} from "~/types/error"

const LAUNCHER_EVENT = "launcher://event"

export type LauncherEvent =
    | (InstallSnapshot & { type: "install" })
    | { type: "instances", instances: Instance[] }
    | { type: "gameStarted", game: RunningGame }
    | { type: "gameStatus", runId: string, instanceId: string, status: RunningGame["status"] }
    | { type: "gameLog", runId: string, instanceId: string, line: string, isError: boolean }
    | { type: "gameExited", runId: string, instanceId: string, code: number | null, logTail?: string }

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

    list_instances: [void, Instance[]]
    reload_instances: [void, Instance[]]
    create_instance: [{ instance: NewInstance }, Instance]
    delete_instance: [{ instanceId: string }, void]

    install_instance: [{ instanceId: string }, InstallSnapshot]
    cancel_install: [{ instanceId: string }, void]
    list_installs: [void, InstallSnapshot[]]

    launch_instance: [{ instanceId: string }, RunningGame]
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

    load_my_packs: [void, MyPacksConfig]
    list_minecraft_versions: [void, VersionManifest]
    list_fabric_versions: [void, string[]]
    list_forge_versions: [void, string[]]
}

export interface NewInstance {
    id: string
    name: string
    description: string
    minecraftVersion: string
    type: Instance["type"]
    version: number
    loaderVersion?: string
    customId?: string
}

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
