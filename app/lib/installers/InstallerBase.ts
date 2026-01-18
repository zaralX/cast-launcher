import type {InstallerProgress, Instance, LivingInstance} from "~/types/instance"
import { ParallelDownloader } from "../ParallelDownloader"
import { path } from "@tauri-apps/api"
import {exists, mkdir, writeTextFile} from "@tauri-apps/plugin-fs";
import {$fetch} from "ofetch";
import {dirname} from "@tauri-apps/api/path";

export abstract class InstallerBase {
    protected instance: LivingInstance
    protected launcherDir: string
    protected librariesDir?: string
    protected assetsDir?: string
    protected cacheDir?: string
    protected minecraftDir?: string

    protected downloader = new ParallelDownloader()
    protected aborted = false

    private progressListeners = new Set<(p: InstallerProgress) => void>()

    constructor(instance: LivingInstance, launcherDir: string) {
        this.instance = instance
        this.launcherDir = launcherDir
    }

    /* ---------- Public API ---------- */

    async install() {
        this.emit({ stage: "prepare", message: "Подготовка" })

        await this.prepare()
        await this.download()
        await this.installFiles()
        await this.finalize()
        await this.finish()

        this.instance.installed = true
    }

    abort() {
        this.emit({ stage: "aborted", message: "Установка прервана" })
        this.aborted = true
    }

    onProgress(cb: (p: InstallerProgress) => void) {
        this.progressListeners.add(cb)
        return () => this.progressListeners.delete(cb)
    }

    /* ---------- Protected helpers ---------- */

    protected emit(progress: InstallerProgress) {
        for (const cb of this.progressListeners) cb(progress)
    }

    protected checkAbort() {
        if (this.aborted) throw new Error("INSTALL_ABORTED")
    }

    /* ---------- Template methods ---------- */

    protected async prepare(): Promise<void> {
        this.librariesDir = await path.join(this.launcherDir, "libraries")
        this.assetsDir = await path.join(this.launcherDir, "assets")
        this.cacheDir = await path.join(this.launcherDir, "cache")
        this.minecraftDir = await path.join(this.instance.dir, "minecraft")

        if (!(await exists(this.librariesDir))) await mkdir(this.librariesDir)
        if (!(await exists(this.assetsDir))) await mkdir(this.assetsDir)
        if (!(await exists(this.cacheDir))) await mkdir(this.cacheDir)
        if (!(await exists(this.minecraftDir))) await mkdir(this.minecraftDir)
    }
    protected abstract download(): Promise<void>
    protected abstract installFiles(): Promise<void>

    protected async downloadJson(url: string, destination: string) {
        const data = await $fetch(url)
        if (!(await exists(await dirname(destination)))) {
            await mkdir(await dirname(destination), {recursive: true})
        }
        await writeTextFile(destination, JSON.stringify(data))
    }

    protected async finalize() {
        this.emit({ stage: "finalize", message: "Завершение установки" })
    }

    protected async finish() {
        this.emit({ stage: "finished", message: "Установка завершена" })
    }
}