import {defineStore} from 'pinia'
import type {AppConfig} from '~/types/app'
import {invoke} from "@tauri-apps/api/core";
import {appConfigDir, dirname} from "@tauri-apps/api/path";
import { path } from "@tauri-apps/api";
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from '@tauri-apps/plugin-process';
import type {AccountConfig} from "~/types/account";

export const useAppStore = defineStore('app', {
    state: () => ({
        config: null as null | AppConfig,
        accountsConfig: null as null | AccountConfig
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
        }
    }
})
