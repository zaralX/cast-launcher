<script setup lang="ts">
import type { SkinEntry, SkinPose, SkinVariant } from '~/types/skin'
import { SOURCE_KEYS, VARIANT_ARM_PIXELS, VARIANT_HINT_KEYS, VARIANT_LABELS } from '~/types/skin'
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

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

const ACCOUNT_BOX = 'flex h-11 w-full items-center gap-2.5 rounded-sm bg-control pr-2.5 pl-1.5 text-left'

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
  <div class="min-h-full w-full px-8 pb-8">
    <div class="flex gap-7">
      <aside class="sticky top-0 flex w-75 shrink-0 flex-col gap-3 self-start pt-6">
        <DropdownMenu v-if="accountItems.length > 1">
          <DropdownMenuTrigger
            :aria-label="$t('skins.account')"
            :class="cn(FOCUS_RING, ACCOUNT_BOX, 'cursor-pointer transition-colors duration-150 hover:bg-hover aria-expanded:bg-hover')"
          >
            <KitAvatar :name="account?.name" />
            <span class="min-w-0 flex-1">
              <span class="block truncate font-medium text-fg">{{ account?.name }}</span>
              <span class="block text-caption text-fg-faint">Microsoft</span>
            </span>
            <Icon
              name="i-lucide-chevrons-up-down"
              class="size-4 text-fg-faint"
            />
          </DropdownMenuTrigger>
          <DropdownMenuContent
            align="start"
            class="w-75"
          >
            <DropdownMenuRadioGroup v-model="activeUuid">
              <DropdownMenuRadioItem
                v-for="item in accountItems"
                :key="item.value"
                :value="item.value"
                class="h-auto gap-3 py-2"
              >
                <KitAvatar :name="item.label" />
                <span class="truncate font-medium">{{ item.label }}</span>
              </DropdownMenuRadioItem>
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>

        <div
          v-else
          :class="ACCOUNT_BOX"
        >
          <KitAvatar :name="account?.name" />
          <span class="min-w-0 flex-1">
            <span class="block truncate font-medium text-fg">{{ account?.name ?? $t('skins.no_license') }}</span>
            <span class="block text-caption text-fg-faint">{{ account ? 'Microsoft' : $t('skins.library_only') }}</span>
          </span>
        </div>

        <Alert
          v-if="demo"
          variant="warning"
          class="text-caption"
        >
          {{ $t('skins.demo_hint_before') }}
          <NuxtLink
            to="/settings?section=accounts"
            class="text-warning underline underline-offset-2"
          >{{ $t('skins.demo_hint_link') }}</NuxtLink>
          {{ $t('skins.demo_hint_after') }}
        </Alert>

        <Alert
          v-else-if="stale"
          icon="i-lucide-cloud-off"
          class="bg-panel text-caption"
        >
          {{ $t('skins.stale') }}
        </Alert>

        <div
          class="relative h-88 overflow-hidden rounded-md ring-1 ring-card-line ring-inset transition-colors duration-300"
          :class="{
            'bg-panel': background === 'ink' || background === 'grid',
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
            class="grid h-full place-items-center px-8 pb-12 text-center text-body text-fg-faint"
          >
            {{ $t('skins.no_set') }}
          </p>

          <div class="absolute bottom-2.5 left-1/2 flex -translate-x-1/2 items-center gap-0.5 rounded-md bg-raise p-0.75 shadow-float ring-1 ring-line-strong">
            <Button
              v-for="item in POSES"
              :key="item.key"
              variant="ghost"
              size="icon-sm"
              :icon="item.icon"
              :title="$t(item.labelKey)"
              :aria-label="$t(item.labelKey)"
              :aria-pressed="pose === item.key"
              @click="pose = item.key"
            />

            <Separator
              orientation="vertical"
              class="mx-1 data-[orientation=vertical]:h-4"
            />

            <Button
              variant="ghost"
              size="icon-sm"
              icon="i-lucide-rotate-3d"
              :title="$t('skins.preview.rotate')"
              :aria-label="$t('skins.preview.rotate')"
              :aria-pressed="spinning"
              @click="spinning = !spinning"
            />

            <Button
              variant="ghost"
              size="icon-sm"
              icon="i-lucide-layers"
              :title="$t('skins.preview.second_layer')"
              :aria-label="$t('skins.preview.second_layer')"
              :aria-pressed="layers"
              @click="layers = !layers"
            />

            <Separator
              orientation="vertical"
              class="mx-1 data-[orientation=vertical]:h-4"
            />

            <Button
              variant="ghost"
              size="icon-sm"
              icon="i-lucide-sun-moon"
              :title="$t('skins.preview.background')"
              :aria-label="$t('skins.preview.background')"
              @click="cycleBackground"
            />

            <Button
              variant="ghost"
              size="icon-sm"
              icon="i-lucide-crosshair"
              :title="$t('skins.preview.reset')"
              :aria-label="$t('skins.preview.reset')"
              @click="model?.reset()"
            />
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
            class="h-7.5"
          >
            {{ VARIANT_LABELS[variant] }}
            <span class="font-mono text-micro font-normal text-fg-faint">{{ VARIANT_ARM_PIXELS[variant] }} px</span>
          </RadioGroupItem>
        </RadioGroup>

        <KitStatus
          v-if="skinStore.dirty && !demo"
          tone="warning"
          class="mt-1"
        >
          {{ $t('skins.pending') }}
        </KitStatus>

        <div class="flex gap-2">
          <Button
            v-if="skinStore.dirty || demo || saving"
            variant="primary"
            size="lg"
            icon="i-lucide-check"
            class="flex-1"
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

          <Button
            v-else
            size="lg"
            icon="i-lucide-check"
            class="flex-1"
            disabled
          >
            {{ $t('skins.applied') }}
          </Button>

          <Button
            v-if="skinStore.dirty && !saving"
            size="icon-lg"
            icon="i-lucide-undo-2"
            :title="$t('skins.revert')"
            :aria-label="$t('skins.revert')"
            @click="skinStore.reset()"
          />
        </div>

        <div
          v-if="!demo"
          class="flex items-center justify-between"
        >
          <Button
            variant="ghost"
            size="sm"
            icon="i-lucide-copy"
            class="-ml-2.5"
            :disabled="!draftSkin"
            @click="draftSkin && duplicate(draftSkin)"
          >
            {{ $t('skins.duplicate') }}
          </Button>

          <Button
            variant="danger-ghost"
            size="sm"
            icon="i-lucide-rotate-ccw"
            class="-mr-2.5"
            @click="resetOpen = true"
          >
            {{ $t('skins.default') }}
          </Button>
        </div>
      </aside>

      <div class="min-w-0 flex-1 space-y-8 pt-6">
        <section class="animate-rise">
          <div class="flex h-11 items-center gap-2">
            <KitSearchInput
              v-model="nickname"
              :placeholder="$t('skins.nickname_placeholder')"
              :aria-label="$t('skins.nickname_placeholder')"
              icon="i-lucide-user-round-search"
              :loading="importingPlayer"
              class="w-72"
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

          <div class="mt-3 grid grid-cols-[repeat(auto-fill,minmax(11.5rem,1fr))] gap-3">
            <div
              v-for="entry in library.skins"
              :key="entry.id"
              class="group/card relative"
              @mouseenter="hovered = entry.id"
              @mouseleave="hovered = null"
            >
              <KitTile
                :selected="draft.skinId === entry.id"
                class="flex w-full flex-col overflow-hidden text-left"
                @click="skinStore.pickSkin(entry.id)"
              >
                <span class="relative block h-50 w-full">
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
                    class="absolute inset-0 m-auto h-24 w-12 rounded-xs bg-control"
                  />

                  <SkinCapeThumb
                    v-if="skinStore.capeById(entry.capeId ?? null)?.texture"
                    :cape="skinStore.capeById(entry.capeId ?? null)!.texture!"
                    :scale="2"
                    class="absolute right-2.5 bottom-2.5 ring-1 ring-line-strong"
                  />

                  <Badge
                    v-if="skinStore.applied.skinId === entry.id"
                    variant="solid"
                    class="absolute top-2.5 left-2.5"
                  >
                    {{ $t('skins.active') }}
                  </Badge>
                </span>

                <span class="block w-full border-t border-line px-3 pt-2 pb-2.5">
                  <span class="block truncate font-medium text-fg">{{ entry.name }}</span>
                  <span class="block truncate text-caption text-fg-faint">
                    {{ VARIANT_LABELS[entry.variant] }} · {{ $t(SOURCE_KEYS[entry.source]) }}
                  </span>
                </span>
              </KitTile>

              <DropdownMenu>
                <DropdownMenuTrigger as-child>
                  <Button
                    size="icon-sm"
                    icon="i-lucide-ellipsis"
                    :title="$t('skins.actions', { name: entry.name })"
                    :aria-label="$t('skins.actions', { name: entry.name })"
                    class="absolute top-2 right-2 bg-raise opacity-0 shadow-float group-hover/card:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
                  />
                </DropdownMenuTrigger>
                <DropdownMenuContent
                  align="start"
                  class="w-56"
                >
                  <DropdownMenuItem
                    icon="i-lucide-pencil"
                    @select="startRename(entry)"
                  >
                    {{ $t('skins.rename') }}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    icon="i-lucide-copy"
                    @select="duplicate(entry)"
                  >
                    {{ $t('skins.duplicate_with_cape') }}
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    icon="i-lucide-trash-2"
                    variant="danger"
                    @select="removeTarget = entry"
                  >
                    {{ $t('skins.remove') }}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>

            <Button
              variant="dashed"
              class="h-auto min-h-63 flex-col gap-2 rounded-md px-4 text-center font-normal whitespace-normal"
              :class="dropping && 'border-accent bg-accent/6 text-accent'"
              @click="importFile()"
            >
              <Icon
                name="i-lucide-image-plus"
                class="size-5.5"
              />
              <span class="font-medium">{{ $t('skins.drop_title') }}</span>
              <span class="text-caption text-fg-faint">{{ $t('skins.drop_sub') }}</span>
            </Button>
          </div>

          <KitEmpty
            v-if="!library.skins.length && !loading"
            class="mt-3 py-10"
          >
            {{ $t('skins.library_empty') }}
          </KitEmpty>
        </section>

        <KitPanel
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
            variant="ghost"
            @click="renameTarget = null"
          >
            {{ $t('common.cancel') }}
          </Button>
          <Button
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
