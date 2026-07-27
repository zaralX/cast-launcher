import {defineStore} from 'pinia'
import {type AppConfig, type JavaMode, type JavaRuntime, CONFIG_VERSION} from '~/types/app'
import {invoke} from "@tauri-apps/api/core";
import {appConfigDir, dirname} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import {check} from "@tauri-apps/plugin-updater";
import {relaunch} from '@tauri-apps/plugin-process';
import type {MyPacksConfig} from "~/types/pack";
import {fetch} from "@tauri-apps/plugin-http";
import {LauncherError, toLauncherError} from "~/types/error";
import {captureError, safeRun} from "~/composables/useErrorHandler";
import {pickJavaRuntime, pickSystemJavaRuntime, type JavaRequirement} from "~/utils/javaUtils";

let javaScan: Promise<JavaRuntime[]> | null = null

export const useAppStore = defineStore('app', {
    state: () => ({
        config: null as null | AppConfig,
        myPacksConfig: null as null | MyPacksConfig,
        javaRuntimes: [] as JavaRuntime[],
        javaScanning: false,
        javaScanned: false,
    }),
    getters: {
        hasConfig: (state) => !!state.config,

        systemJavaRuntime: (state): JavaRuntime | null => pickSystemJavaRuntime(state.javaRuntimes),
    },
    actions: {
        async getConfigPath() {
            return await path.join(
                await appConfigDir(),
                "config.json"
            )
        },

        async getDefaultConfig() {
            return {
                version: CONFIG_VERSION,
                launcher: {
                    language: "ru",
                    theme: "dark",
                    dir: await dirname(await this.getConfigPath()),
                    auto_update: true
                },
                java: {
                    java_mode: "auto" as JavaMode,
                    java_path: "",
                    min_ram: 1024,
                    max_ram: 4096
                }
            }
        },

        async loadConfig() {
            const configPath = await this.getConfigPath()
            const defaults = await this.getDefaultConfig()

            if (!(await exists(configPath))) {
                defaults.launcher.dir = await dirname(configPath)
                this.config = defaults
                await this.updateConfig(this.config)
                return this.config
            }

            let raw: any = null
            try {
                raw = JSON.parse(await readTextFile(configPath))
            } catch (e) {
                captureError(e, {code: "CONFIG_ERROR", context: {path: configPath}})
            }

            const migrated = this.migrateConfig(raw ?? {})
            const merged = this.mergeConfig(defaults, migrated)

            this.config = merged
            await this.updateConfig(this.config)

            console.log("Loaded config ", this.config)

            return this.config
        },

        async updateConfig(config: AppConfig) {
            const configPath = await this.getConfigPath()

            try {
                if (!(await exists(configPath))) {
                    await mkdir(await dirname(configPath), {recursive: true})
                }
                await writeTextFile(configPath, JSON.stringify(config))
            } catch (e) {
                throw toLauncherError(e, "FS_ERROR", {path: configPath})
            }

            this.config = config
        },

        migrateConfig(config: any): AppConfig {
            let cfg = {...config}

            if (!cfg.version) {
                cfg.version = 1
            }

            // Cfg 1 -> 2 Migration
            if (cfg.version === 1) {
                cfg.launcher = {
                    ...cfg.launcher,
                    auto_update: true
                }
                cfg.version = 2
            }

            // Cfg 2 -> 3 Migration
            if (cfg.version === 2) {
                cfg.java = {
                    ...cfg.java,
                    // Указанный руками путь остаётся, остальные переезжают на подбор по версии
                    java_mode: cfg.java?.java_path ? "manual" : "auto"
                }
                cfg.version = 3
            }

            return cfg
        },

        mergeConfig(defaults: AppConfig, user: any): AppConfig {
            return {
                ...defaults,
                ...user,
                launcher: {
                    ...defaults.launcher,
                    ...user.launcher
                },
                java: {
                    ...defaults.java,
                    ...user.java
                }
            }
        },


        async scanJava(force = false): Promise<JavaRuntime[]> {
            if (javaScan) return await javaScan
            if (this.javaScanned && !force) return this.javaRuntimes

            this.javaScanning = true
            javaScan = invoke<JavaRuntime[]>("list_java")

            try {
                this.javaRuntimes = await javaScan
                this.javaScanned = true
                console.log("Found java runtimes ", this.javaRuntimes)
            } catch (e) {
                throw toLauncherError(e, "JAVA_NOT_FOUND", {})
            } finally {
                javaScan = null
                this.javaScanning = false
            }

            return this.javaRuntimes
        },

        async probeJava(path: string): Promise<JavaRuntime | null> {
            if (!path.trim()) return null
            return await invoke<JavaRuntime | null>("probe_java", {path})
        },

        // required — какая Java нужна сборке; учитывается только в режиме auto
        async resolveJavaPath(required: JavaRequirement | null = null): Promise<string> {
            const mode = this.config?.java?.java_mode ?? "auto"
            const manual = this.config?.java?.java_path?.trim()

            if (mode === "manual" && manual) return manual

            if (!this.javaScanned) {
                await safeRun(() => this.scanJava())
            }

            if (mode === "auto") {
                const picked = pickJavaRuntime(this.javaRuntimes, required)
                if (picked) return picked.path

                // Подходящей нет — берём системную, чтобы дойти хотя бы до понятной ошибки запуска
                console.warn(`Java ${required?.major} для сборки не найдена, используем системную`)
            }

            return this.systemJavaRuntime?.path ?? "java"
        },

        async updateApp() {
            let update
            try {
                update = await check();
            } catch (e) {
                throw toLauncherError(e, "UPDATE_FAILED", {})
            }

            if (update) {
                console.log(
                    `found update ${update.version} from ${update.date} with notes ${update.body}`
                );
                let downloaded = 0;
                let contentLength = 0;
                // alternatively we could also call update.download() and update.install() separately
                await update.downloadAndInstall((event) => {
                    switch (event.event) {
                        case 'Started':
                            contentLength = event.data.contentLength as number;
                            console.log(`started downloading ${event.data.contentLength} bytes`);
                            break;
                        case 'Progress':
                            downloaded += event.data.chunkLength;
                            console.log(`downloaded ${downloaded} from ${contentLength}`);
                            break;
                        case 'Finished':
                            console.log('download finished');
                            break;
                    }
                });

                console.log('update installed');
                await relaunch();
            }
        },
        async loadMyPacks() {
            const url = "https://s3.zaralx.ru/launcher/my_packs.json"

            let response: Response
            try {
                response = await fetch(url)
            } catch (e) {
                throw toLauncherError(e, "NETWORK", {url})
            }

            if (!response.ok) {
                throw new LauncherError("NETWORK", {
                    message: `Список сборок недоступен (HTTP ${response.status})`,
                    context: {url}
                })
            }

            try {
                this.myPacksConfig = await response.json() as MyPacksConfig
            } catch (e) {
                throw toLauncherError(e, "MANIFEST_INVALID", {url})
            }

            console.log("Loaded myPacksConfig ", this.myPacksConfig)
        },
    }
})
