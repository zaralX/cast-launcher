import type {
    InstallerProgress,
    Instance,
    LivingInstance,
    MinecraftEvent,
    MinecraftLogEvent,
    MinecraftStatus,
    MinecraftStatusEvent
} from "~/types/instance"
import { ParallelDownloader } from "../ParallelDownloader"
import { path } from "@tauri-apps/api"
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import {$fetch} from "ofetch";
import {dirname} from "@tauri-apps/api/path";
import {arch, platform} from "@tauri-apps/plugin-os";
import type {Account} from "~/types/account";
import {invoke} from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {v4} from "uuid";
import {type ErrorContext, LauncherError, toLauncherError} from "~/types/error";

const LOG_TAIL = 60;

export abstract class ClientBase {
    public instance: LivingInstance
    public id: string // used for backend emits
    public versionType?: string

    protected launcherDir: string
    protected librariesDir?: string
    protected assetsDir?: string
    protected nativesDir?: string
    protected minecraftDir?: string

    private unlistenLog?: () => void
    private unlistenStatus?: () => void
    private unlistenExit?: () => void

    private listeners = new Set<(e: MinecraftEvent) => void>()
    private recentLogs: string[] = []

    constructor(launcherDir: string, instance: LivingInstance) {
        this.instance = instance
        this.launcherDir = launcherDir
        this.id = v4()
    }

    public errorContext(extra: ErrorContext = {}): ErrorContext {
        return {
            instanceId: this.instance.id,
            instanceName: this.instance.name,
            minecraftVersion: this.instance.minecraftVersion,
            loader: this.instance.type,
            loaderVersion: this.instance.loaderVersion,
            ...extra
        }
    }

    public getLogTail(): string {
        return this.recentLogs.join("\n")
    }

    onEvent(cb: (e: MinecraftEvent) => void) {
        this.listeners.add(cb)
        return () => this.listeners.delete(cb)
    }

    protected emit(e: MinecraftEvent) {
        for (const cb of this.listeners) cb(e)
    }

    public async prepare() {
        this.librariesDir = await path.join(this.launcherDir, "libraries")
        this.assetsDir = await path.join(this.launcherDir, "assets")
        this.minecraftDir = await path.join(this.instance.dir, "minecraft")
        this.nativesDir = await path.join(this.minecraftDir, "natives")

        try {
            if (!(await exists(this.nativesDir))) await mkdir(this.nativesDir, { recursive: true })
        } catch (e) {
            throw toLauncherError(e, "FS_ERROR", this.errorContext({ path: this.nativesDir }))
        }
    }

    protected async readInstalledJson<T = any>(file: string, what: string): Promise<T> {
        try {
            return JSON.parse(await readTextFile(file)) as T
        } catch (e) {
            throw new LauncherError("FS_ERROR", {
                message: `Не найден или повреждён ${what}. Переустановите сборку.`,
                details: `${file}\n${e instanceof Error ? e.message : String(e)}`,
                context: this.errorContext({ path: file }),
                cause: e
            })
        }
    }

    protected async getFullArgs(account: Account): Promise<string[]> {
        return []
    }

    protected async generateCP(libraries: any[]): Promise<string[]> {
        const cp: string[] = []
        for (const library of libraries) {
            if (!library?.downloads?.artifact?.path) continue // Skips only native libs
            cp.push(await path.join(this.librariesDir!, library.downloads.artifact.path))
        }
        cp.push(await path.join(this.minecraftDir!, "client.jar"))

        return cp
    }

    protected async generateArgs(placeholders: Record<string, any> = {}): Promise<string[]> {
        return []
    }

    public static getMojangRuleFilteredArgs(args: any[]): string[] {
        const filteredArgs: string[] = []

        const os = platform();
        const architecture = arch();

        for (const arg of args as any[]) {
            if (typeof arg == 'string') {
                filteredArgs.push(arg)
            } else {
                if (arg.rules) {
                    const allow = checkRules(arg.rules, os, architecture)
                    if (allow) {
                        if (typeof arg.value == 'string') {
                            filteredArgs.push(arg.value)
                        } else if (Array.isArray(arg.value)) {
                            filteredArgs.push(...arg.value)
                        }
                    }
                }
            }
        }

        return filteredArgs
    }

    public static replaceArgPlaceholders(args: string[], placeholders: Record<string, any>): string[] {
        return args.map(str =>
            str.replace(/\$\{(\w+)}/g, (match, key) => placeholders[key] ?? match)
        )
    }

    public async run(javaPath: string, account: Account) {
        await this.injectListeners()

        try {
            const args = await this.getFullArgs(account)
            console.log("Starting minecraft", this.instance, account, args)
            await invoke("launch_minecraft", {
                javaPath: javaPath,
                clientId: this.id,
                args: args
            });
        } catch (e) {
            this.destroyListeners()
            throw toLauncherError(e, "LAUNCH_FAILED", this.errorContext({ javaPath }))
        }
    }

    // isError - stderr / stdout detection
    protected onLog(line: string, isError: boolean) {
        this.recentLogs.push(line)
        if (this.recentLogs.length > LOG_TAIL) this.recentLogs.shift()

        this.emit({
            type: 'log',
            line,
        })
        console.log(this.id, line)
    }

    protected onStatus(status: MinecraftStatus) {
        this.emit({
            type: 'status',
            status
        })
        console.log(this.id, "Minecraft status changed:", status)
    }

    protected onExit(code: number | null) {
        this.emit({
            type: 'exit',
            code
        })
        console.log(this.id, "Minecraft exited with code", code)
        this.destroyListeners()
    }

    public crashError(code: number | null): LauncherError {
        return new LauncherError("LAUNCH_FAILED", {
            message: `Minecraft завершился с кодом ${code ?? "неизвестно"}`,
            details: this.getLogTail(),
            context: this.errorContext({ exitCode: code })
        })
    }

    protected async injectListeners() {
        this.unlistenLog = await listen<MinecraftLogEvent>(
            `${this.id}:log`,
            e => this.onLog(e.payload.line, e.payload.is_error)
        )

        this.unlistenStatus = await listen<MinecraftStatusEvent>(
            `${this.id}:status`,
            e => this.onStatus(e.payload.status)
        )

        this.unlistenExit = await listen<number | null>(
            `${this.id}:exit`,
            e => this.onExit(e.payload)
        )
    }

    protected destroyListeners() {
        this.unlistenLog?.()
        this.unlistenStatus?.()
        this.unlistenExit?.()
    }

}