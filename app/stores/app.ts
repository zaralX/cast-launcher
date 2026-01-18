import {defineStore} from 'pinia'
import type {AppConfig} from '~/types/app'
import {invoke} from "@tauri-apps/api/core";

export const useAppStore = defineStore('app', {
    state: () => ({
        config: null as null | AppConfig,
    }),
    getters: {
        hasConfig: (state) => !!state.config,
    },
    actions: {
        async loadConfig() {
            this.config = await invoke("get_config")
        },

        async updateConfig(config: AppConfig) {
            await invoke("update_config", {newConfig: config})
        },
    }
})
