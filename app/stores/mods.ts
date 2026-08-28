import {defineStore} from 'pinia'
import type {
    CatalogMatch,
    CatalogVersion,
    InstallPlan,
    InstallReport,
    InstalledMods,
    ModFile,
    ModSearchQuery,
    ModUpdate,
    UpdatedModsReport
} from '~/types/mods'
import type {PackSearchPage} from '~/types/catalog'
import {call} from '~/types/backend'

const pendingIcons = new Map<string, Promise<string | null>>()

export const useModsStore = defineStore('mods', {
    state: () => ({
        byInstance: {} as Record<string, ModFile[]>,
        loading: [] as string[],
        icons: {} as Record<string, string>,
        catalog: {} as Record<string, Record<string, CatalogMatch>>,
        updates: {} as Record<string, ModUpdate[]>,
        identifying: [] as string[],
        checking: [] as string[]
    }),
    getters: {
        listOf: (state) => (instanceId: string): ModFile[] => state.byInstance[instanceId] ?? [],
        isLoading: (state) => (instanceId: string) => state.loading.includes(instanceId),
        isLoaded: (state) => (instanceId: string) => !!state.byInstance[instanceId],
        iconOf: (state) => (key?: string | null): string | null => (key && state.icons[key]) || null,
        matchOf: (state) => (instanceId: string, path: string): CatalogMatch | null =>
            state.catalog[instanceId]?.[path] ?? null,
        updatesOf: (state) => (instanceId: string): ModUpdate[] => state.updates[instanceId] ?? [],
        isIdentifying: (state) => (instanceId: string) => state.identifying.includes(instanceId),
        isChecking: (state) => (instanceId: string) => state.checking.includes(instanceId)
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

        async identify(instanceId: string): Promise<Record<string, CatalogMatch>> {
            if (this.identifying.includes(instanceId)) return this.catalog[instanceId] ?? {}

            this.identifying.push(instanceId)

            try {
                const matches = await call("identify_instance_mods", {instanceId})

                this.catalog[instanceId] = matches

                return matches
            } finally {
                this.identifying = this.identifying.filter(id => id !== instanceId)
            }
        },

        async checkUpdates(instanceId: string): Promise<ModUpdate[]> {
            if (this.checking.includes(instanceId)) return this.updatesOf(instanceId)

            this.checking.push(instanceId)

            try {
                const updates = await call("check_mod_updates", {instanceId})

                this.updates[instanceId] = updates

                return updates
            } finally {
                this.checking = this.checking.filter(id => id !== instanceId)
            }
        },

        async update(instanceId: string, paths: string[]): Promise<UpdatedModsReport> {
            const {mods, report} = await call("update_mods", {instanceId, paths})

            this.byInstance[instanceId] = mods
            this.updates[instanceId] = this.updatesOf(instanceId).filter(update => !paths.includes(update.path))

            return report
        },

        async searchMods(instanceId: string, query: ModSearchQuery): Promise<PackSearchPage> {
            return await call("search_mods", {instanceId, query})
        },

        async modVersions(
            instanceId: string,
            provider: CatalogMatch["provider"],
            projectId: string
        ): Promise<CatalogVersion[]> {
            return await call("mod_versions", {instanceId, provider, projectId})
        },

        async planInstall(
            instanceId: string,
            provider: CatalogMatch["provider"],
            projectId: string,
            versionId: string
        ): Promise<InstallPlan> {
            return await call("plan_mod_install", {instanceId, provider, projectId, versionId})
        },

        async installMod(instanceId: string, planId: string, optional: string[]): Promise<InstallReport> {
            const {mods, report} = await call("install_mod", {instanceId, planId, optional})

            this.byInstance[instanceId] = mods

            return report
        },

        forget(instanceId: string) {
            delete this.byInstance[instanceId]
            delete this.catalog[instanceId]
            delete this.updates[instanceId]
        }
    }
})
