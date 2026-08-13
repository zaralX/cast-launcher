import {defineStore} from 'pinia'
import type {Account, AccountConfig} from "~/types/account"
import {call} from "~/types/backend"

export const useAccountStore = defineStore('account', {
    state: () => ({
        accountConfig: null as null | AccountConfig,
        loggingIn: false
    }),
    getters: {
        hasConfig: (state) => !!state.accountConfig,

        accounts: (state): Account[] => state.accountConfig?.accounts ?? [],

        selected: (state): Account | null => {
            const accounts = state.accountConfig?.accounts ?? []
            return accounts[state.accountConfig?.selected ?? 0] ?? null
        }
    },
    actions: {
        applyBootstrap(config: AccountConfig) {
            this.accountConfig = config
        },

        async reload() {
            this.accountConfig = await call("list_accounts")
        },

        async selectAccount(index: number) {
            this.accountConfig = await call("select_account", {index})
        },

        async addOfflineAccount(name: string) {
            this.accountConfig = await call("add_offline_account", {name})
        },

        async removeAccount(uuid: string) {
            this.accountConfig = await call("remove_account", {uuid})
        },

        async microsoftLogin() {
            if (this.loggingIn) return

            this.loggingIn = true

            try {
                await call("login_microsoft")
                this.accountConfig = await call("list_accounts")
            } finally {
                this.loggingIn = false
            }
        },

        async refreshAccount(uuid: string) {
            await call("refresh_account", {uuid})
            this.accountConfig = await call("list_accounts")
        }
    }
})
