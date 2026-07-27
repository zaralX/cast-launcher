import type {
    DownloadFileProgress,
    InstallerProgress,
    InstallerStage,
    InstallPhase,
    Instance,
    LivingInstance,
    MojangLibraryNative
} from "~/types/instance"
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

    private phases: InstallPhase[] = []
    private phaseIndex = -1
    private phaseMessage = ""
    private overall = 0

    constructor(instance: LivingInstance, launcherDir: string, javaPath: string = "java") {
        this.instance = instance
        this.launcherDir = launcherDir
        this.javaPath = javaPath
    }

    async install() {
        this.phases = this.definePhases()
        this.phaseIndex = -1
        this.phaseMessage = "Подготовка"
        this.overall = 0

        this.emit({ stage: "prepare", type: "global", message: "Подготовка", progress: 0 })

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

    protected definePhases(): InstallPhase[] {
        return [{ key: "install", label: "Установка", weight: 1 }]
    }

    protected beginPhase(key: string, message: string) {
        const index = this.phases.findIndex(p => p.key === key)
        if (index >= 0) this.phaseIndex = index
        this.phaseMessage = message
        this.emitOverall(0)
    }

    protected reportPhase(fraction: number, message?: string) {
        if (message) this.phaseMessage = message
        this.emitOverall(fraction)
    }

    protected emitFile(file: DownloadFileProgress, label: string) {
        this.emit({
            stage: this.stage,
            type: "single",
            message: `${label} · ${file.name}`,
            progress: file.percent,
            file
        })
    }

    private emitOverall(fraction: number) {
        const phase = this.phases[this.phaseIndex]
        const totalWeight = this.phases.reduce((a, p) => a + p.weight, 0)

        if (phase && totalWeight > 0) {
            const before = this.phases
                .slice(0, this.phaseIndex)
                .reduce((a, p) => a + p.weight, 0)
            const value = (before + Math.min(1, Math.max(0, fraction)) * phase.weight) / totalWeight
            this.overall = Math.min(1, Math.max(this.overall, value))
        }

        this.emit({
            stage: this.stage,
            type: "global",
            message: this.phaseMessage,
            phase: phase?.label,
            progress: this.overall
        })
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
        this.emit({ stage: "finalize", type: "global", message: "Завершение установки", progress: this.overall })

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
        this.overall = 1
        this.emit({ stage: "finished", type: "global", message: "Установка завершена", progress: 1 })
    }
}