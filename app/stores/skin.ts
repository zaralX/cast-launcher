import type { AccountLook, CapeView, Look, SkinEntry, SkinLibrary, SkinVariant } from '~/types/skin'

export const COOLDOWN_MS = 30_000

const emptyLibrary = (): SkinLibrary => ({ skins: [] })

const emptyLook = (): Look => ({ skinId: null, capeId: null, variant: 'CLASSIC' })

const texturesInFlight = new Map<string, Promise<void>>()

export const useSkinStore = defineStore('skin', {
  state: () => ({
    loading: false,
    refreshingUuid: null as string | null,
    revision: 0,
    saving: false,
    uuid: null as string | null,
    name: '',
    stale: false,
    library: emptyLibrary(),
    capes: [] as CapeView[],
    applied: emptyLook(),
    draft: emptyLook(),
    previous: null as Look | null,
    cooldownUntil: 0,
    textures: {} as Record<string, string>,
  }),

  getters: {
    skins: (state): SkinEntry[] => state.library.skins,

    skinById: state => (id: string | null) =>
      state.library.skins.find(entry => entry.id === id) ?? null,

    capeById: state => (id: string | null) =>
      state.capes.find(cape => cape.id === id) ?? null,

    draftSkin(): SkinEntry | null {
      return this.skinById(this.draft.skinId)
    },

    draftCape(): CapeView | null {
      return this.capeById(this.draft.capeId)
    },

    textureOf: state => (entry: SkinEntry | null) =>
      entry ? state.textures[entry.texture] ?? null : null,

    draftTexture(): string | null {
      return this.textureOf(this.draftSkin)
    },

    dirty: state => !!state.uuid && !sameLook(state.draft, state.applied),

    refreshing: state => !!state.uuid && state.refreshingUuid === state.uuid,
  },

  actions: {
    apply(look: AccountLook) {
      this.uuid = look.uuid
      this.name = look.name
      this.stale = look.stale
      this.library = look.library
      this.capes = look.capes
      this.applied = lookOf(look)
      this.draft = lookOf(look)
      this.revision++

      this.ensureTextures()
    },

    merge(look: AccountLook) {
      const untouched = sameLook(this.draft, this.applied)

      this.name = look.name
      this.stale = look.stale
      this.library = look.library
      this.capes = look.capes
      this.applied = lookOf(look)

      if (untouched) this.draft = lookOf(look)

      this.ensureTextures()
    },

    applyLibrary(library: SkinLibrary) {
      this.library = library

      if (!this.skinById(this.draft.skinId)) this.select(library.skins[0] ?? null)

      this.ensureTextures()
    },

    async load(uuid: string) {
      if (this.uuid !== uuid) await this.loadSaved(uuid)

      await this.refresh(uuid)
    },

    async loadSaved(uuid: string) {
      this.uuid = uuid
      this.stale = false
      this.applied = emptyLook()
      this.draft = emptyLook()
      this.loading = true

      try {
        const look = await call('account_look', { uuid, refresh: false })
        if (this.uuid !== uuid) return

        this.apply(look)
        await this.ensureDraftTexture()
      }
      catch (error) {
        if (this.uuid === uuid) this.uuid = null
        throw error
      }
      finally {
        if (this.uuid === uuid || !this.uuid) this.loading = false
      }
    },

    async refresh(uuid: string) {
      if (this.refreshingUuid === uuid) return

      this.refreshingUuid = uuid
      const revision = this.revision

      try {
        const look = await call('account_look', { uuid, refresh: true })
        if (this.uuid !== uuid || this.revision !== revision) return

        this.merge(look)
        await this.ensureDraftTexture()
      }
      finally {
        if (this.refreshingUuid === uuid) this.refreshingUuid = null
      }
    },

    async loadLibrary() {
      this.applyLibrary(await call('skin_library'))
    },

    async ensureTexture(texture: string) {
      if (this.textures[texture]) return

      let pending = texturesInFlight.get(texture)

      if (!pending) {
        pending = call('skin_texture', { texture })
          .then((url) => {
            this.textures[texture] = url
          })
          .finally(() => texturesInFlight.delete(texture))

        texturesInFlight.set(texture, pending)
      }

      await pending
    },

    async ensureDraftTexture() {
      const entry = this.draftSkin
      if (entry) await this.ensureTexture(entry.texture).catch(() => undefined)
    },

    ensureTextures() {
      for (const entry of this.library.skins) {
        if (!this.textures[entry.texture]) safeRun(() => this.ensureTexture(entry.texture))
      }
    },

    select(entry: SkinEntry | null) {
      this.draft = {
        skinId: entry?.id ?? null,
        capeId: entry?.capeId ?? null,
        variant: entry?.variant ?? 'CLASSIC',
      }
    },

    pickSkin(id: string) {
      this.select(this.skinById(id))
    },

    async pickCape(capeId: string | null) {
      this.draft.capeId = capeId

      const id = this.draft.skinId
      if (!id) return

      this.library = await call('set_skin_cape', { id, capeId })
    },

    async setVariant(variant: SkinVariant) {
      this.draft.variant = variant

      const id = this.draft.skinId
      if (!id) return

      this.library = await call('set_skin_variant', { id, variant })
    },

    async duplicate(id: string) {
      const name = useNuxtApp().$i18n.t('skins.copy_name', { name: this.skinById(id)?.name ?? '' })
      const entry = await call('duplicate_skin', { id, capeId: this.draft.capeId, name })

      await this.loadLibrary()
      this.select(this.skinById(entry.id))

      return entry
    },

    async importFile(path?: string) {
      const { $i18n } = useNuxtApp()
      const dialog = { title: $i18n.t('dialog.skin.title'), filter: $i18n.t('dialog.skin.filter') }
      const entry = await call('import_skin', { path, dialog })

      if (entry) {
        await this.loadLibrary()
        this.select(this.skinById(entry.id))
      }

      return entry
    },

    async importPlayer(name: string) {
      const entry = await call('import_player_skin', { name })

      await this.loadLibrary()
      this.select(this.skinById(entry.id))

      return entry
    },

    async rename(id: string, name: string) {
      this.applyLibrary(await call('rename_skin', { id, name }))
    },

    async remove(id: string) {
      this.applyLibrary(await call('delete_skin', { id }))
    },

    async save() {
      if (this.saving || !this.uuid) return false

      const uuid = this.uuid
      const before = { ...this.applied }
      const target = { ...this.draft }

      this.saving = true

      try {
        if (target.skinId && target.skinId !== this.applied.skinId) {
          this.apply(await call('apply_skin', { uuid, id: target.skinId }))
        }

        if (target.capeId !== this.applied.capeId) {
          this.apply(await call('apply_cape', { uuid, capeId: target.capeId }))
        }

        this.previous = before
        this.cooldownUntil = Date.now() + COOLDOWN_MS

        return true
      }
      finally {
        this.saving = false
      }
    },

    async undo() {
      if (!this.previous) return false

      this.draft = { ...this.previous }
      this.previous = null

      return await this.save()
    },

    async resetSkin() {
      if (!this.uuid) return

      this.saving = true

      try {
        this.apply(await call('reset_skin', { uuid: this.uuid }))
        this.cooldownUntil = Date.now() + COOLDOWN_MS
      }
      finally {
        this.saving = false
      }
    },

    reset() {
      this.draft = { ...this.applied }
    },
  },
})
