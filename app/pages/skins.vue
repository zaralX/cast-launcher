<script setup lang="ts">
import type { SkinEntry, SkinPose, SkinVariant } from '~/types/skin'
import { SOURCE_KEYS, VARIANT_HINT_KEYS, VARIANT_LABELS } from '~/types/skin'

definePageMeta({
  layout: 'main',
})

const toast = useAppToast()

const { t } = useI18n()
const skinStore = useSkinStore()
const { library, capes, draft, loading, saving, stale } = storeToRefs(skinStore)

const accountStore = useAccountStore()
const { accountConfig } = storeToRefs(accountStore)

const licensed = computed(() => (accountConfig.value?.accounts ?? []).filter(item => item.type === 'microsoft'))

const activeUuid = ref<string>()

watch(licensed, (list) => {
  if (!activeUuid.value || !list.some(item => item.uuid === activeUuid.value)) {
    activeUuid.value = list[0]?.uuid
  }
}, { immediate: true })

const account = computed(() => licensed.value.find(item => item.uuid === activeUuid.value) ?? null)
const demo = computed(() => !account.value)

const accountItems = computed(() => licensed.value.map(item => ({
  label: item.name,
  value: item.uuid ?? item.name,
})))

const POSES: { key: SkinPose, icon: string, labelKey: string }[] = [
  { key: 'stand', icon: 'i-lucide-user-round', labelKey: 'skins.pose.stand' },
  { key: 'walk', icon: 'i-lucide-footprints', labelKey: 'skins.pose.walk' },
  { key: 'run', icon: 'i-lucide-wind', labelKey: 'skins.pose.run' },
]

const BACKGROUNDS = ['ink', 'grid', 'light'] as const
type Background = typeof BACKGROUNDS[number]

const pose = ref<SkinPose>('walk')
const spinning = ref(false)
const layers = ref(true)
const background = ref<Background>('grid')

const model = useTemplateRef('model')

const draftSkin = computed(() => skinStore.draftSkin)
const draftTexture = computed(() => skinStore.draftTexture)
const draftCape = computed(() => skinStore.draftCape)

function cycleBackground() {
  const next = BACKGROUNDS.indexOf(background.value) + 1
  background.value = BACKGROUNDS[next % BACKGROUNDS.length]!
}

const pickCape = (capeId: string | null) =>
  safeRun(() => skinStore.pickCape(capeId), { context: { action: t('skins.cape_action') } })

const setVariant = (variant: SkinVariant) =>
  safeRun(() => skinStore.setVariant(variant), { context: { action: t('skins.variant_action') } })

const importing = ref(false)

const hovered = ref<string | null>(null)

async function importFile(path?: string) {
  if (importing.value) return

  importing.value = true

  const result = await attempt(() => skinStore.importFile(path), { context: { action: t('skins.import_action') } })

  importing.value = false

  if (result.ok && result.value) {
    toast.add({ title: t('skins.imported', { name: result.value.name }), color: 'success', icon: 'i-lucide-image-plus' })
  }
}

const dropping = useFileDrop((paths) => {
  const png = paths.find(path => path.toLowerCase().endsWith('.png'))

  if (!png) {
    toast.add({ title: t('skins.need_png'), description: t('skins.need_png_hint'), color: 'error', icon: 'i-lucide-file-x' })
    return
  }

  importFile(png)
})

const nickname = ref('')
const importingPlayer = ref(false)

async function importPlayer() {
  const name = nickname.value.trim()
  if (!name || importingPlayer.value) return

  importingPlayer.value = true

  const result = await attempt(() => skinStore.importPlayer(name), { context: { action: t('skins.player_action') } })

  importingPlayer.value = false

  if (result.ok) {
    nickname.value = ''
    toast.add({ title: t('skins.player_loaded', { name: result.value.name }), color: 'success', icon: 'i-lucide-user-round-search' })
  }
}

async function duplicate(entry: SkinEntry) {
  const result = await attempt(() => skinStore.duplicate(entry.id), { context: { action: t('skins.duplicate_action') } })

  if (result.ok) {
    toast.add({
      title: t('skins.duplicated', { name: result.value.name }),
      color: 'success',
      icon: 'i-lucide-copy',
    })
  }
}

const renameTarget = ref<SkinEntry | null>(null)
const renameValue = ref('')

function startRename(entry: SkinEntry) {
  renameTarget.value = entry
  renameValue.value = entry.name
}

async function commitRename() {
  const target = renameTarget.value
  if (!target) return

  renameTarget.value = null

  await safeRun(() => skinStore.rename(target.id, renameValue.value), { context: { action: t('skins.rename_action') } })
}

const removeTarget = ref<SkinEntry | null>(null)

async function confirmRemove() {
  const target = removeTarget.value
  if (!target) return

  removeTarget.value = null

  const result = await attempt(() => skinStore.remove(target.id), { context: { action: t('skins.remove_action') } })

  if (result.ok) {
    toast.add({ title: t('skins.removed', { name: target.name }), color: 'success', icon: 'i-lucide-trash-2' })
  }
}

const now = ref(Date.now())
let ticker: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  ticker = setInterval(() => now.value = Date.now(), 500)
})

onBeforeUnmount(() => {
  if (ticker) clearInterval(ticker)
})

const cooldown = computed(() => Math.max(0, Math.ceil((skinStore.cooldownUntil - now.value) / 1000)))

const canSave = computed(() => !demo.value && skinStore.dirty && !cooldown.value)

async function save() {
  if (demo.value) {
    toast.add({
      title: t('skins.need_account_title'),
      description: t('skins.need_account_hint'),
      color: 'error',
      icon: 'i-lucide-lock',
    })
    return false
  }

  const result = await attempt(() => skinStore.save(), { context: { action: t('skins.save_action') } })

  if (!result.ok || !result.value) return false

  toast.add({
    title: t('skins.saved', { name: skinStore.name }),
    description: t('skins.saved_hint'),
    color: 'success',
    icon: 'i-lucide-check',
    actions: [{
      label: t('skins.undo'),
      onClick: () => undo(),
    }],
  })

  return true
}

async function undo() {
  const result = await attempt(() => skinStore.undo(), { context: { action: t('skins.undo_action') } })

  if (result.ok && result.value) {
    toast.add({ title: t('skins.undone'), color: 'success', icon: 'i-lucide-undo-2' })
  }
}

const resetOpen = ref(false)

async function resetSkin() {
  resetOpen.value = false

  const result = await attempt(() => skinStore.resetSkin(), { context: { action: t('skins.reset_action') } })

  if (result.ok) toast.add({ title: t('skins.reset_done'), color: 'success', icon: 'i-lucide-rotate-ccw' })
}

const guard = useUnsavedChanges({
  dirty: () => skinStore.dirty,
  canSave,
  save,
  discard: () => skinStore.reset(),
})

const VARIANTS: SkinVariant[] = ['CLASSIC', 'SLIM']

async function reload(uuid?: string) {
  if (uuid) {
    await safeRun(() => skinStore.load(uuid), { context: { action: t('skins.load_action') } })
    return
  }

  await safeRun(() => skinStore.loadLibrary(), { context: { action: t('skins.library_action') } })
}

onMounted(() => reload(activeUuid.value))

watch(activeUuid, uuid => reload(uuid))
</script>

<template>
  <div class="min-h-full w-full px-8 pb-8 xl:px-14">
    <div class="grid grid-cols-[18rem_minmax(0,1fr)] gap-12 xl:grid-cols-[22rem_minmax(0,1fr)]">
      <aside class="sticky top-0 space-y-4 self-start pt-10">
        <div class="flex items-center gap-3 border border-line bg-ink-800 px-4 py-3">
          <img
            v-if="account"
            :src="`https://assets.zaralx.ru/api/v2/minecraft/players/${account.name}/face`"
            class="size-7 shrink-0"
            :alt="account.name"
            @error="fallbackFace"
          >
          <span
            v-else
            class="grid size-7 shrink-0 place-items-center border border-line text-fg-faint"
          >
            <Icon
              name="i-lucide-user-round"
              class="size-3.5"
            />
          </span>

          <div class="min-w-0 flex-1">
            <KitSelect
              v-if="accountItems.length > 1"
              v-model="activeUuid"
              :items="accountItems"
            />
            <template v-else>
              <p class="truncate text-title text-fg">
                {{ account?.name ?? $t('skins.no_license') }}
              </p>
              <p class="mt-0.5 font-mono text-micro uppercase tracking-caps text-fg-faint">
                {{ account ? 'Microsoft' : $t('skins.library_only') }}
              </p>
            </template>
          </div>
        </div>

        <Alert
          v-if="demo"
          variant="warning"
          class="text-caption"
        >
          {{ $t('skins.demo_hint_before') }}
          <NuxtLink
            to="/settings"
            class="text-warning underline underline-offset-2"
          >{{ $t('skins.demo_hint_link') }}</NuxtLink>
          {{ $t('skins.demo_hint_after') }}
        </Alert>

        <Alert
          v-else-if="stale"
          icon="i-lucide-cloud-off"
          class="bg-ink-800 text-caption"
        >
          {{ $t('skins.stale') }}
        </Alert>

        <div
          class="cut-16 relative h-72 border border-line transition-colors duration-500 xl:h-92"
          :class="{
            'bg-ink-800': background === 'ink',
            'bg-ink-900': background === 'grid',
            'bg-fg-muted/90': background === 'light',
          }"
        >
          <div
            v-if="background === 'grid'"
            class="pointer-events-none absolute inset-0 opacity-[0.06]"
            style="background-image: linear-gradient(currentColor 1px, transparent 1px), linear-gradient(90deg, currentColor 1px, transparent 1px); background-size: 16px 16px"
            aria-hidden="true"
          />

          <div
            v-if="loading"
            class="grid h-full place-items-center"
          >
            <Progress
              :model-value="null"
              class="w-32"
            />
          </div>

          <SkinModel
            v-else-if="draftTexture"
            ref="model"
            class="absolute inset-x-0 top-0 bottom-12"
            :skin="draftTexture"
            :cape="draftCape?.texture ?? null"
            :variant="draft.variant"
            :pose="pose"
            :spinning="spinning"
            :layers="layers"
          />

          <p
            v-else
            class="grid h-full place-items-center px-8 text-center font-mono text-label uppercase tracking-caps text-fg-faint"
          >
            {{ $t('skins.no_set') }}
          </p>

          <div class="absolute inset-x-0 bottom-0 flex items-center justify-between border-t border-line bg-ink-900/70 p-2 backdrop-blur">
            <div class="flex items-center gap-0.5">
              <Button
                v-for="item in POSES"
                :key="item.key"
                variant="toolbar"
                size="icon-sm"
                :icon="item.icon"
                :title="$t(item.labelKey)"
                :aria-label="$t(item.labelKey)"
                :aria-pressed="pose === item.key"
                @click="pose = item.key"
              />

              <Separator
                orientation="vertical"
                class="mx-1 h-4"
              />

              <Button
                variant="toolbar"
                size="icon-sm"
                icon="i-lucide-rotate-3d"
                :title="$t('skins.preview.rotate')"
                :aria-label="$t('skins.preview.rotate')"
                :aria-pressed="spinning"
                @click="spinning = !spinning"
              />

              <Button
                variant="toolbar"
                size="icon-sm"
                icon="i-lucide-layers"
                :title="$t('skins.preview.second_layer')"
                :aria-label="$t('skins.preview.second_layer')"
                :aria-pressed="layers"
                @click="layers = !layers"
              />
            </div>

            <div class="flex items-center gap-0.5">
              <Button
                variant="toolbar"
                size="icon-sm"
                icon="i-lucide-sun-moon"
                :title="$t('skins.preview.background')"
                :aria-label="$t('skins.preview.background')"
                @click="cycleBackground"
              />

              <Button
                variant="toolbar"
                size="icon-sm"
                icon="i-lucide-crosshair"
                :title="$t('skins.preview.reset')"
                :aria-label="$t('skins.preview.reset')"
                @click="model?.reset()"
              />
            </div>
          </div>
        </div>

        <RadioGroup
          :model-value="draft.variant"
          @update:model-value="value => setVariant(value as SkinVariant)"
        >
          <RadioGroupItem
            v-for="variant in VARIANTS"
            :key="variant"
            :value="variant"
            :title="$t(VARIANT_HINT_KEYS[variant])"
            class="px-4 py-2.5 font-mono text-label uppercase tracking-caps"
          >
            {{ VARIANT_LABELS[variant] }}
          </RadioGroupItem>
        </RadioGroup>

        <Button
          size="xl"
          icon="i-lucide-check"
          class="w-full"
          :loading="saving"
          :disabled="!canSave"
          @click="save"
        >
          <template v-if="saving">
            {{ $t('skins.applying') }}
          </template>
          <template v-else-if="cooldown">
            {{ $t('skins.cooldown', { seconds: cooldown }) }}
          </template>
          <template v-else>
            {{ $t('skins.apply') }}
          </template>
        </Button>

        <div
          v-if="!demo"
          class="flex items-center justify-between gap-3"
        >
          <Button
            variant="quiet"
            icon="i-lucide-copy"
            :disabled="!draftSkin"
            @click="draftSkin && duplicate(draftSkin)"
          >
            {{ $t('skins.duplicate') }}
          </Button>

          <Button
            variant="quiet-danger"
            icon="i-lucide-rotate-ccw"
            @click="resetOpen = true"
          >
            {{ $t('skins.default') }}
          </Button>
        </div>

        <div
          v-if="skinStore.dirty"
          class="flex justify-end"
        >
          <Button
            variant="quiet"
            @click="skinStore.reset()"
          >
            {{ $t('skins.revert') }}
          </Button>
        </div>
      </aside>

      <div class="space-y-6 pt-10">
        <KitPanel
          index="01"
          :title="$t('skins.library_title')"
          icon="i-lucide-shirt"
          class="animate-rise"
        >
          <div class="space-y-5">
            <div class="flex items-center gap-2">
              <KitSearchInput
                v-model="nickname"
                :placeholder="$t('skins.nickname_placeholder')"
                icon="i-lucide-user-round-search"
                :loading="importingPlayer"
                size="lg"
                class="w-44"
                @keyup.enter="importPlayer"
              />

              <Button
                icon="i-lucide-upload"
                :loading="importing"
                @click="importFile()"
              >
                {{ $t('skins.file') }}
              </Button>
            </div>

            <div class="grid grid-cols-3 gap-2.5 xl:grid-cols-4 2xl:grid-cols-6">
              <KitTile
                v-for="entry in library.skins"
                :key="entry.id"
                :selected="draft.skinId === entry.id"
                class="group/card flex aspect-3/4 flex-col overflow-hidden"
                @click="skinStore.pickSkin(entry.id)"
                @mouseenter="hovered = entry.id"
                @mouseleave="hovered = null"
              >
                <div class="relative min-h-0 flex-1">
                  <SkinModel
                    v-if="skinStore.textureOf(entry)"
                    class="absolute inset-0"
                    :skin="skinStore.textureOf(entry)!"
                    :cape="skinStore.capeById(entry.capeId ?? null)?.texture ?? null"
                    :variant="entry.variant"
                    :pose="hovered === entry.id ? 'walk' : 'stand'"
                    :angle="30"
                    :scale="0.9"
                    :interactive="false"
                  />
                  <span
                    v-else
                    class="absolute inset-0 m-auto h-16 w-8 bg-line/40"
                  />

                  <SkinCapeThumb
                    v-if="skinStore.capeById(entry.capeId ?? null)?.texture"
                    :cape="skinStore.capeById(entry.capeId ?? null)!.texture!"
                    :scale="1.5"
                    class="absolute right-1 bottom-1 border border-line/60"
                  />
                </div>

                <div class="w-full border-t border-line px-2 py-1.5 text-left">
                  <p class="truncate text-label leading-tight text-fg">
                    {{ entry.name }}
                  </p>
                  <p class="mt-0.5 truncate font-mono text-[8px] uppercase tracking-caps text-fg-faint">
                    {{ VARIANT_LABELS[entry.variant] }} · {{ $t(SOURCE_KEYS[entry.source]) }}
                  </p>
                </div>

                <Badge
                  v-if="skinStore.applied.skinId === entry.id"
                  variant="solid"
                  class="absolute top-0 left-0 text-[7px]"
                >
                  {{ $t('skins.active') }}
                </Badge>

                <div class="absolute top-1 right-1 hidden gap-0.5 group-hover/card:flex">
                  <span
                    class="grid size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 hover:border-acid/50 hover:text-acid"
                    :title="$t('skins.duplicate_with_cape')"
                    @click.stop="duplicate(entry)"
                  >
                    <Icon
                      name="i-lucide-copy"
                      class="size-2.5"
                    />
                  </span>

                  <span
                    class="grid size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 hover:border-acid/50 hover:text-acid"
                    :title="$t('skins.rename')"
                    @click.stop="startRename(entry)"
                  >
                    <Icon
                      name="i-lucide-pencil"
                      class="size-2.5"
                    />
                  </span>

                  <span
                    class="grid size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 hover:border-danger/50 hover:text-danger"
                    :title="$t('skins.remove')"
                    @click.stop="removeTarget = entry"
                  >
                    <Icon
                      name="i-lucide-trash-2"
                      class="size-2.5"
                    />
                  </span>
                </div>
              </KitTile>

              <button
                type="button"
                class="group/drop flex aspect-3/4 cursor-pointer flex-col items-center justify-center gap-2 border border-dashed outline-none transition-colors duration-300 focus-visible:border-line-strong"
                :class="dropping ? 'border-acid bg-acid/6' : 'border-line hover:border-line-strong hover:bg-ink-700'"
                @click="importFile()"
              >
                <Icon
                  name="i-lucide-image-plus"
                  class="size-4 text-fg-faint transition-colors duration-300 group-hover/drop:text-acid"
                />
                <i18n-t
                  keypath="skins.drop_hint"
                  tag="span"
                  class="px-2 text-center font-mono text-[8px] uppercase leading-relaxed tracking-caps text-fg-faint"
                >
                  <template #br>
                    <br>
                  </template>
                </i18n-t>
              </button>
            </div>

            <KitEmpty
              v-if="!library.skins.length && !loading"
              class="py-10"
            >
              {{ $t('skins.library_empty') }}
            </KitEmpty>
          </div>
        </KitPanel>

        <KitPanel
          index="02"
          :title="draftSkin ? $t('skins.cape_title_named', { name: draftSkin.name }) : $t('skins.cape_title')"
          icon="i-lucide-flag"
          class="animate-rise [animation-delay:80ms]"
        >
          <KitEmpty
            v-if="!draftSkin"
            class="py-8"
          >
            {{ $t('skins.pick_set_first') }}
          </KitEmpty>

          <div
            v-else-if="capes.length"
            class="flex flex-wrap gap-3"
          >
            <KitTile
              :selected="draft.capeId === null"
              class="flex h-26 w-18 flex-col items-center justify-center gap-2"
              @click="pickCape(null)"
            >
              <Icon
                name="i-lucide-ban"
                class="size-4 text-fg-faint"
              />
              <span class="font-mono text-[8px] uppercase tracking-caps text-fg-faint">{{ $t('skins.no_cape') }}</span>
            </KitTile>

            <KitTile
              v-for="cape in capes"
              :key="cape.id"
              :selected="draft.capeId === cape.id"
              class="flex h-26 w-18 flex-col items-center justify-center gap-2"
              :title="cape.alias"
              @click="pickCape(cape.id)"
            >
              <SkinCapeThumb
                v-if="cape.texture"
                :cape="cape.texture"
                :scale="5"
                class="transition-transform duration-500 ease-deck"
              />
            </KitTile>
          </div>

          <KitEmpty
            v-else
            class="py-8"
          >
            {{ demo ? $t('skins.need_microsoft') : $t('skins.no_capes') }}
          </KitEmpty>
        </KitPanel>
      </div>
    </div>

    <Dialog
      :open="!!renameTarget"
      @update:open="value => { if (!value) renameTarget = null }"
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ $t('skins.rename_title') }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <Input
            v-model="renameValue"
            autofocus
            @keyup.enter="commitRename"
          />
        </DialogBody>
        <DialogFooter>
          <Button
            variant="quiet"
            @click="renameTarget = null"
          >
            {{ $t('common.cancel') }}
          </Button>
          <Button
            size="sm"
            icon="i-lucide-check"
            @click="commitRename"
          >
            {{ $t('common.save') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <KitConfirmDialog
      :open="!!removeTarget"
      :title="$t('skins.remove_title')"
      :description="$t('skins.remove_text', { name: removeTarget?.name })"
      :confirm-label="$t('common.delete')"
      @update:open="value => { if (!value) removeTarget = null }"
      @confirm="confirmRemove"
    />

    <KitConfirmDialog
      v-model:open="resetOpen"
      tone="default"
      :title="$t('skins.reset_title')"
      :description="$t('skins.reset_text')"
      :confirm-label="$t('skins.reset')"
      confirm-icon="i-lucide-rotate-ccw"
      :loading="saving"
      @confirm="resetSkin"
    />

    <AppUnsavedChangesModal
      :guard="guard"
      :description="$t('skins.leave.description')"
      :blocked="$t('skins.leave.blocked')"
      :discard-label="$t('skins.leave.discard')"
    />
  </div>
</template>
