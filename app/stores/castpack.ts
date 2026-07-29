import {defineStore} from 'pinia'
import type {Catalog, CatalogPack, CastPackUpdate} from '~/types/castpack'
import type {Instance} from '~/types/instance'
import {call} from '~/types/backend'
import {useInstanceStore} from '~/stores/instance'

export type PackState = "absent" | "installing" | "outdated" | "ready" | "running"

export const useCastPackStore = defineStore('castpack', {
    state: () => ({
        catalog: null as Catalog | null,
        loading: false,
        loaded: false,
        updates: {} as Record<string, CastPackUpdate>,
        installing: [] as string[]
    }),
    getters: {
        packs: (state): CatalogPack[] => state.catalog?.packs ?? [],

        instanceOf() {
            const instances = useInstanceStore()

            return (packId: string): Instance | undefined =>
                instances.instances.find(instance => instance.castpack?.catalogId === packId)
        },

        stateOf(): (pack: CatalogPack) => PackState {
            const instances = useInstanceStore()

            return (pack: CatalogPack) => {
                const instance = this.instanceOf(pack.id)

                if (!instance || !instance.installed) {
                    return this.installing.includes(pack.id) || (instance && instances.getInstall(instance.id))
                        ? "installing"
                        : "absent"
                }

                if (instances.getInstall(instance.id)) return "installing"
                if (instances.isRunning(instance.id)) return "running"

                const installed = instance.castpack?.version ?? ""

                return pack.version && installed && pack.version !== installed ? "outdated" : "ready"
            }
        }
    },
    actions: {
        async loadCatalog(force = false) {
            if (this.loading) return this.catalog
            if (this.loaded && !force) return this.catalog

            this.loading = true

            try {
                this.catalog = await call("castpack_catalog")
                this.loaded = true
            } finally {
                this.loading = false
            }

            return this.catalog
        },

        async installPack(packId: string) {
            if (this.installing.includes(packId)) return

            this.installing.push(packId)

            try {
                return await call("castpack_install", {packId})
            } finally {
                this.installing = this.installing.filter(id => id !== packId)
            }
        },

        async checkUpdate(instanceId: string) {
            const update = await call("castpack_check_update", {instanceId})
            this.updates[instanceId] = update

            return update
        },

        async setAutoupdate(instanceId: string, enabled: boolean) {
            const instance = await call("castpack_set_autoupdate", {instanceId, enabled})
            const instances = useInstanceStore()

            const index = instances.instances.findIndex(item => item.id === instanceId)
            if (index >= 0) instances.instances[index] = instance

            return instance
        }
    }
})
