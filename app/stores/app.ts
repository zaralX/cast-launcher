import {defineStore} from 'pinia'
import type {AppConfig} from '~/types/app'
import {invoke} from "@tauri-apps/api/core";
import {appConfigDir, dirname} from "@tauri-apps/api/path";
import { path } from "@tauri-apps/api";
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";

export const useAppStore = defineStore('app', {
    state: () => ({
        config: null as null | AppConfig,
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

        async loadConfig() {
            const configPath = await this.getConfigPath()
            if (!(await exists(configPath))) {
                await mkdir(await dirname(configPath), {recursive: true})
                this.config = {
                    launcher: {
                        language: "ru",
                        theme: "dark",
                        dir: await dirname(configPath)
                    },
                    java: {
                        java_path: "",
                        min_ram: 1024,
                        max_ram: 4096
                    }
                }
                await writeTextFile(configPath, JSON.stringify(this.config))
            } else {
                this.config = JSON.parse(await readTextFile(configPath))
            }

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
    }
})
