import type {InstallerProgress, InstallerStage, Instance, LivingInstance, MojangLibraryNative} from "~/types/instance"
import { ParallelDownloader } from "../ParallelDownloader"
import { path } from "@tauri-apps/api"
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import {$fetch} from "ofetch";
import {dirname} from "@tauri-apps/api/path";
import {invoke} from "@tauri-apps/api/core";
import {type ErrorContext, LauncherError, toLauncherError} from "~/types/error";

export abstract class InstallerBase {
    protected instance: LivingInstance
    protected launcherDir: string
    protected javaPath: string
    protected librariesDir?: string
    protected assetsDir?: string
    protected cacheDir?: string
    protected minecraftDir?: string
    protected nativesDir?: string

    protected downloader = new ParallelDownloader()
    protected aborted = false
    protected stage: InstallerStage = "prepare"

    private progressListeners = new Set<(p: InstallerProgress) => void>()

    constructor(instance: LivingInstance, launcherDir: string, javaPath: string = "java") {
        this.instance = instance
        this.launcherDir = launcherDir
        this.javaPath = javaPath
    }

    /* ---------- Public API ---------- */

    async install() {
        this.emit({ stage: "prepare", message: "Подготовка" })

        try {
            await this.prepare()
            this.checkAbort()

            await this.download()
            this.checkAbort()

            await this.installFiles()
            this.checkAbort()

            await this.finalize()
            await this.finish()

            this.instance.installed = true
        } catch (raw) {
            const error = this.wrap(raw)

            if (error.code === "INSTALL_ABORTED") {
                this.emit({ stage: "aborted", message: error.title, error })
            } else {
                this.emit({ stage: "failed", message: error.title, error })
            }

            throw error
        }
    }

    abort() {
        this.aborted = true
    }

    onProgress(cb: (p: InstallerProgress) => void) {
        this.progressListeners.add(cb)
        return () => this.progressListeners.delete(cb)
    }

    /* ---------- Protected helpers ---------- */

    protected emit(progress: InstallerProgress) {
        this.stage = progress.stage
        for (const cb of this.progressListeners) cb(progress)
    }

    protected errorContext(extra: ErrorContext = {}): ErrorContext {
        return {
            instanceId: this.instance.id,
            instanceName: this.instance.name,
            minecraftVersion: this.instance.minecraftVersion,
            loader: this.instance.type,
            loaderVersion: this.instance.loaderVersion,
            stage: this.stage,
            ...extra
        }
    }

    protected wrap(raw: unknown, extra: ErrorContext = {}): LauncherError {
        return toLauncherError(raw, "UNKNOWN", this.errorContext(extra))
    }

    protected checkAbort() {
        if (this.aborted) {
            throw new LauncherError("INSTALL_ABORTED", { context: this.errorContext() })
        }
    }

    /* ---------- Template methods ---------- */

    protected async prepare(): Promise<void> {
        this.librariesDir = await path.join(this.launcherDir, "libraries")
        this.assetsDir = await path.join(this.launcherDir, "assets")
        this.cacheDir = await path.join(this.launcherDir, "cache")
        this.minecraftDir = await path.join(this.instance.dir, "minecraft")
        this.nativesDir = await path.join(this.minecraftDir, "natives")

        for (const dir of [this.librariesDir, this.assetsDir, this.cacheDir, this.minecraftDir, this.nativesDir]) {
            try {
                if (!(await exists(dir))) await mkdir(dir, { recursive: true })
            } catch (e) {
                throw this.wrap(e, { path: dir })
            }
        }
    }
    protected abstract download(): Promise<void>
    protected abstract installFiles(): Promise<void>

    protected async fetchJson<T = any>(url: string): Promise<T> {
        try {
            return await $fetch<T>(url)
        } catch (e) {
            throw toLauncherError(e, "NETWORK", this.errorContext({ url }))
        }
    }

    protected async downloadJson(url: string, destination: string) {
        const data = await this.fetchJson(url)

        try {
            if (!(await exists(await dirname(destination)))) {
                await mkdir(await dirname(destination), {recursive: true})
            }
            await writeTextFile(destination, JSON.stringify(data))
        } catch (e) {
            throw this.wrap(e, { path: destination, url })
        }
    }

    protected async readCachedJson<T = any>(file: string): Promise<T | null> {
        try {
            if (!(await exists(file))) return null
            return JSON.parse(await readTextFile(file)) as T
        } catch (e) {
            console.warn("Failed to read cached json, refetching", file, e)
            return null
        }
    }

    protected async installNative(
        native: MojangLibraryNative,
        destination: string
    ) {
        const nativeJarPath = await path.join(this.librariesDir!, native.path)

        if (! (await exists(nativeJarPath)) ) {
            throw new LauncherError("FS_ERROR", {
                message: `Нативная библиотека не найдена: ${native.path}`,
                context: this.errorContext({ path: nativeJarPath })
            })
        }

        try {
            await invoke("extract_jar", {
                jarPath: nativeJarPath,
                outputDir: destination
            })
        } catch (e) {
            throw this.wrap(e, { path: nativeJarPath })
        }
    }

    protected async finalize() {
        this.emit({ stage: "finalize", message: "Завершение установки" })

        const staticInstanceFile = await path.join(this.instance.dir, "instance.json")

        try {
            const staticInstanceData: Instance = JSON.parse(await readTextFile(staticInstanceFile))
            staticInstanceData.installed = true
            await writeTextFile(staticInstanceFile, JSON.stringify(staticInstanceData))
        } catch (e) {
            throw this.wrap(e, { path: staticInstanceFile })
        }
    }

    protected async finish() {
        this.emit({ stage: "finished", message: "Установка завершена" })
    }
}