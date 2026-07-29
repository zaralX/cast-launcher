import {defineStore} from 'pinia'
import {check} from "@tauri-apps/plugin-updater"
import {relaunch} from '@tauri-apps/plugin-process'
import type {AppConfig, JavaRuntime, LauncherPaths} from '~/types/app'
import {call} from "~/types/backend"

export const useAppStore = defineStore('app', {
    state: () => ({
        config: null as null | AppConfig,
        paths: null as null | LauncherPaths,
        javaRuntimes: [] as JavaRuntime[],
        javaScanning: false,
        javaScanned: false
    }),
    getters: {
        hasConfig: (state) => !!state.config,

        systemJavaRuntime: (state): JavaRuntime | null =>
            state.javaRuntimes.find(runtime => runtime.source === "path" || runtime.source === "java_home")
            ?? state.javaRuntimes[0]
            ?? null
    },
    actions: {
        applyBootstrap(config: AppConfig, paths: LauncherPaths) {
            this.config = config
            this.paths = paths
        },

        async updateConfig(config: AppConfig) {
            this.config = await call("update_config", {config})
            this.paths = await call("get_paths")

            this.javaScanned = false
        },

        async scanJava(force = false): Promise<JavaRuntime[]> {
            if (this.javaScanning) return this.javaRuntimes
            if (this.javaScanned && !force) return this.javaRuntimes

            this.javaScanning = true

            try {
                this.javaRuntimes = await call("list_java", {force})
                this.javaScanned = true
            } finally {
                this.javaScanning = false
            }

            return this.javaRuntimes
        },

        async probeJava(path: string): Promise<JavaRuntime | null> {
            if (!path.trim()) return null
            return await call("probe_java", {path})
        },

        async openLauncherDir() {
            if (this.paths) await call("open_path", {path: this.paths.root})
        },

        async updateApp() {
            const update = await check({timeout: 15000})
            if (!update) return false

            await update.downloadAndInstall()
            await relaunch()

            return true
        }
    }
})
