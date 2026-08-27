import {defineStore} from 'pinia'
import type {InstalledMods, ModFile} from '~/types/mods'
import {call} from '~/types/backend'

const pendingIcons = new Map<string, Promise<string | null>>()

export const useModsStore = defineStore('mods', {
    state: () => ({
        byInstance: {} as Record<string, ModFile[]>,
        loading: [] as string[],
        icons: {} as Record<string, string>
    }),
    getters: {
        listOf: (state) => (instanceId: string): ModFile[] => state.byInstance[instanceId] ?? [],
        isLoading: (state) => (instanceId: string) => state.loading.includes(instanceId),
        isLoaded: (state) => (instanceId: string) => !!state.byInstance[instanceId],
        iconOf: (state) => (key?: string | null): string | null => (key && state.icons[key]) || null
    },
    actions: {
        async load(instanceId: string, force = false): Promise<ModFile[]> {
            if (!instanceId) return []
            if (this.byInstance[instanceId] && !force) return this.byInstance[instanceId]!
            if (this.loading.includes(instanceId)) return this.byInstance[instanceId] ?? []

            this.loading.push(instanceId)

            try {
                const mods = await call(force ? "refresh_instance_mods" : "list_instance_mods", {instanceId})

                this.byInstance[instanceId] = mods

                return mods
            } finally {
                this.loading = this.loading.filter(id => id !== instanceId)
            }
        },

        async ensureIcon(key?: string | null): Promise<string | null> {
            if (!key) return null
            if (this.icons[key]) return this.icons[key]!

            const pending = pendingIcons.get(key)
            if (pending) return await pending

            const request = call("read_mod_icon", {key})
                .then(url => {
                    this.icons[key] = url
                    return url
                })
                .catch(() => null)
                .finally(() => pendingIcons.delete(key))

            pendingIcons.set(key, request)

            return await request
        },

        async setEnabled(instanceId: string, path: string, enabled: boolean): Promise<ModFile[]> {
            const mods = await call("set_mod_enabled", {instanceId, path, enabled})

            this.byInstance[instanceId] = mods

            return mods
        },

        async remove(instanceId: string, paths: string[]): Promise<ModFile[]> {
            const mods = await call("delete_mods", {instanceId, paths})

            this.byInstance[instanceId] = mods

            return mods
        },

        async add(instanceId: string, paths: string[]): Promise<InstalledMods> {
            const {mods, report} = await call("add_mods", {instanceId, paths})

            this.byInstance[instanceId] = mods

            return report
        },

        forget(instanceId: string) {
            delete this.byInstance[instanceId]
        }
    }
})
