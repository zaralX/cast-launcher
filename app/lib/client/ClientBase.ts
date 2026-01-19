import type {InstallerProgress, Instance, LivingInstance} from "~/types/instance"
import { ParallelDownloader } from "../ParallelDownloader"
import { path } from "@tauri-apps/api"
import {exists, mkdir, writeTextFile} from "@tauri-apps/plugin-fs";
import {$fetch} from "ofetch";
import {dirname} from "@tauri-apps/api/path";
import {arch, platform} from "@tauri-apps/plugin-os";
import type {MinecraftAccount} from "~/types/account";
import {invoke} from "@tauri-apps/api/core";

export abstract class ClientBase {
    protected instance: LivingInstance
    protected launcherDir: string
    protected librariesDir?: string
    protected assetsDir?: string
    protected nativesDir?: string
    protected minecraftDir?: string

    constructor(launcherDir: string, instance: LivingInstance) {
        this.instance = instance
        this.launcherDir = launcherDir
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
        const args = await this.getFullArgs(account)
        console.log("Starting minecraft", this.instance, account, args)
        await invoke("launch_minecraft", {
            javaPath: "C:/Users/admin/AppData/Roaming/PrismLauncher/java/java-runtime-delta/bin/javaw.exe",
            args: args
        });
    }
}