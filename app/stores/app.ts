import {defineStore} from 'pinia'
import {type AppConfig, CONFIG_VERSION} from '~/types/app'
import {appConfigDir, dirname} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import {check} from "@tauri-apps/plugin-updater";
import {relaunch} from '@tauri-apps/plugin-process';
import type {MyPacksConfig} from "~/types/pack";
import {fetch} from "@tauri-apps/plugin-http";

export const useAppStore = defineStore('app', {
    state: () => ({
        config: null as null | AppConfig,
        myPacksConfig: null as null | MyPacksConfig,
    }),
    getters: {
        hasConfig: (state) => !!state.config,
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

            const raw = JSON.parse(await readTextFile(configPath))

            const migrated = this.migrateConfig(raw)
            const merged = this.mergeConfig(defaults, migrated)

            this.config = merged
            await this.updateConfig(this.config)

            console.log("Loaded config ", this.config)

            return this.config
        },

        async updateConfig(config: AppConfig) {
            const configPath = await this.getConfigPath()
            if (!(await exists(configPath))) {
                await mkdir(await dirname(configPath), {recursive: true})
            }
            await writeTextFile(configPath, JSON.stringify(config))
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


        async updateApp() {
            const update = await check();
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
            this.myPacksConfig = await (await fetch("https://s3.zaralx.ru/launcher/my_packs.json")).json() as MyPacksConfig
            console.log("Loaded myPacksConfig ", this.myPacksConfig)
        },
    }
})
