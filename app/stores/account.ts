import {defineStore} from 'pinia'
import type {AppConfig} from '~/types/app'
import {invoke} from "@tauri-apps/api/core";
import {appConfigDir, dirname} from "@tauri-apps/api/path";
import { path } from "@tauri-apps/api";
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from '@tauri-apps/plugin-process';
import type {AccountConfig} from "~/types/account";

export const useAccountStore = defineStore('account', {
    state: () => ({
        config: null as null | AccountConfig,
    }),
    getters: {
        hasConfig: (state) => !!state.config,
    },
    actions: {
        async getConfigPath() {
            return await path.join(
                await appConfigDir(),
                "accounts.json"
            )
        },

        async loadConfig() {
            const configPath = await this.getConfigPath()
            if (!(await exists(configPath))) {
                await mkdir(await dirname(configPath), {recursive: true})
                this.config = {
                    accounts: []
                }
                await writeTextFile(configPath, JSON.stringify(this.config))
            } else {
                this.config = JSON.parse(await readTextFile(configPath))
            }

            return this.config
        },

        async updateConfig(config: AccountConfig) {
            const configPath = await this.getConfigPath()
            if (!(await exists(configPath))) {
                await mkdir(await dirname(configPath), {recursive: true})
            }
            await writeTextFile(configPath, JSON.stringify(config))
            this.config = config
        },
    }
})
