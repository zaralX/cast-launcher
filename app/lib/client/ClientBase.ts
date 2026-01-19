import type {InstallerProgress, Instance, LivingInstance, MinecraftLogEvent, MinecraftStatusEvent} from "~/types/instance"
import { ParallelDownloader } from "../ParallelDownloader"
import { path } from "@tauri-apps/api"
import {exists, mkdir, writeTextFile} from "@tauri-apps/plugin-fs";
import {$fetch} from "ofetch";
import {dirname} from "@tauri-apps/api/path";
import {arch, platform} from "@tauri-apps/plugin-os";
import type {MinecraftAccount} from "~/types/account";
import {invoke} from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {v4} from "uuid";

export abstract class ClientBase {
    protected instance: LivingInstance
    protected id: string // used for backend emits

    protected launcherDir: string
    protected librariesDir?: string
    protected assetsDir?: string
    protected nativesDir?: string
    protected minecraftDir?: string

    private unlistenLog?: () => void
    private unlistenStatus?: () => void
    private unlistenExit?: () => void

    constructor(launcherDir: string, instance: LivingInstance) {
        this.instance = instance
        this.launcherDir = launcherDir
        this.id = v4()
    }

    public async prepare() {
        this.librariesDir = await path.join(this.launcherDir, "libraries")
        this.assetsDir = await path.join(this.launcherDir, "assets")
        this.minecraftDir = await path.join(this.instance.dir, "minecraft")
        this.nativesDir = await path.join(this.minecraftDir, "natives")
        if (!(await exists(this.nativesDir))) await mkdir(this.nativesDir)
    }

    protected async getFullArgs(account: MinecraftAccount): Promise<string[]> {
        return []
    }

    protected async generateCP(libraries: any[]): Promise<string[]> {
        const cp: string[] = []
        for (const library of libraries) {
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

    public async run(account: MinecraftAccount) {
        await this.injectListeners()
        const args = await this.getFullArgs(account)
        console.log("Starting minecraft", this.instance, account, args)
        await invoke("launch_minecraft", {
            javaPath: "C:/Users/admin/AppData/Roaming/PrismLauncher/java/java-runtime-delta/bin/javaw.exe",
            clientId: this.id,
            args: args
        });
    }

    protected onLog(line: string, isError: boolean) {
        console.log(isError ? "[MC STDERR]" : "[MC STDOUT]", line)
    }

    protected onStatus(status: MinecraftStatusEvent["status"]) {
        console.log("Minecraft status:", status)
    }

    protected onExit(code: number | null) {
        console.log("Minecraft exited with code", code)
        this.destroyListeners()
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