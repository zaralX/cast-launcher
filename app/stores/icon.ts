import {defineStore} from 'pinia'
import type {IconFile, ItemCatalog} from '~/types/icon'
import {call} from '~/types/backend'

const pendingLibrary = new Map<string, Promise<string | null>>()
const pendingItems = new Set<string>()

export const useIconStore = defineStore('icon', {
    state: () => ({
        library: [] as IconFile[],
        libraryLoaded: false,
        urls: {} as Record<string, string>,
        catalog: null as ItemCatalog | null,
        catalogLoading: false,
        itemUrls: {} as Record<string, string>
    }),
    getters: {
        has: (state) => (name: string) => !!name && state.library.some(icon => icon.name === name),
        urlOf: (state) => (name: string): string | null => (name && state.urls[name]) || null,
        itemUrlOf: (state) => (item: string): string | null => state.itemUrls[item] ?? null
    },
    actions: {
        async loadLibrary(force = false) {
            if (this.libraryLoaded && !force) return this.library

            this.library = await call("list_icons")
            this.libraryLoaded = true

            return this.library
        },

        async ensureUrl(name: string): Promise<string | null> {
            if (!name) return null
            if (this.urls[name]) return this.urls[name]

            const pending = pendingLibrary.get(name)
            if (pending) return await pending

            const request = call("read_icon", {name})
                .then(url => {
                    this.urls[name] = url
                    return url
                })
                .catch(() => null)
                .finally(() => pendingLibrary.delete(name))

            pendingLibrary.set(name, request)

            return await request
        },

        forgetUrl(name: string) {
            delete this.urls[name]
        },

        forgetLibrary() {
            this.library = []
            this.libraryLoaded = false
            this.urls = {}
        },

        async importFile(): Promise<IconFile | null> {
            const icon = await call("import_icon", {})
            if (!icon) return null

            await this.loadLibrary(true)
            this.forgetUrl(icon.name)
            await this.ensureUrl(icon.name)

            return icon
        },

        async removeIcon(name: string) {
            this.library = await call("delete_icon", {name})
            this.forgetUrl(name)
        },

        async loadCatalog(force = false) {
            if (this.catalog && !force) return this.catalog
            if (this.catalogLoading) return this.catalog

            this.catalogLoading = true

            try {
                this.catalog = await call("list_item_icons")
            } finally {
                this.catalogLoading = false
            }

            return this.catalog
        },

        async ensureItemUrls(items: string[]) {
            const missing = items.filter(item => !this.itemUrls[item] && !pendingItems.has(item))
            if (!missing.length) return

            missing.forEach(item => pendingItems.add(item))

            try {
                const fetched = await call("item_icons", {items: missing})
                this.itemUrls = {...this.itemUrls, ...fetched}
            } finally {
                missing.forEach(item => pendingItems.delete(item))
            }
        },

        async useItem(item: string): Promise<IconFile> {
            const icon = await call("save_item_icon", {item})

            await this.loadLibrary(true)
            await this.ensureUrl(icon.name)

            return icon
        }
    }
})
