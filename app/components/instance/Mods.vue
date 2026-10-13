<script setup lang="ts">
import type { ComponentPublicInstance } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { useResizeObserver } from '@vueuse/core'
import type { Instance } from '~/types/instance'
import type { CatalogMatch, InstalledMods, ModFile, ModUpdate } from '~/types/mods'
import { CATALOG_LABELS, MOD_LOADER_LABELS } from '~/types/mods'
import { PROVIDER_LOGOS } from '~/types/catalog'

const props = defineProps<{ instance: Instance }>()

const ROW_HEIGHT = 57

type Filter = 'all' | 'enabled' | 'disabled' | 'outdated'

const modsStore = useModsStore()
const toast = useAppToast()

const instanceId = computed(() => props.instance.id)

const mods = computed(() => modsStore.listOf(instanceId.value))
const loading = computed(() => modsStore.isLoading(instanceId.value))
const loaded = computed(() => modsStore.isLoaded(instanceId.value))

const query = ref('')
const filter = ref<Filter>('all')
const catalogOpen = ref(false)
const expanded = ref('')
const selected = ref<string[]>([])
const busy = ref<string[]>([])
const working = ref(false)
const { t } = useI18n()

const confirming = ref(false)

const identifying = computed(() => modsStore.isIdentifying(instanceId.value))
const checking = computed(() => modsStore.isChecking(instanceId.value))

const updates = computed(() => modsStore.updatesOf(instanceId.value))

const matchOf = (path: string): CatalogMatch | null => modsStore.matchOf(instanceId.value, path)
const updateOf = (path: string): ModUpdate | null => updates.value.find(update => update.path === path) ?? null

const disabledCount = computed(() => mods.value.filter(mod => !mod.enabled).length)

const FILTERS = computed(() => [
  { label: t('instance.mods.filter.all'), value: 'all', count: mods.value.length },
  { label: t('instance.mods.filter.enabled'), value: 'enabled', count: mods.value.length - disabledCount.value },
  { label: t('instance.mods.filter.disabled'), value: 'disabled', count: disabledCount.value },
  { label: t('instance.mods.filter.outdated'), value: 'outdated', count: updates.value.length },
])

const GRID = 'grid-cols-[1rem_2.25rem_minmax(0,1fr)_9.5rem_7.5rem_2.25rem_1.75rem]'

function askRemove(mod: ModFile) {
  selected.value = [modKey(mod)]
  confirming.value = true
}

const visible = computed(() => mods.value.filter((mod) => {
  if (filter.value === 'enabled' && !mod.enabled) return false
  if (filter.value === 'disabled' && mod.enabled) return false
  if (filter.value === 'outdated' && !updateOf(mod.path)) return false

  return matchesMod(mod, query.value, matchOf(mod.path))
}))

const root = ref<HTMLElement | null>(null)
const list = ref<HTMLElement | null>(null)
const scroller = ref<HTMLElement | null>(null)
const listOffset = ref(0)

const virtualizer = useVirtualizer(computed(() => {
  const items = visible.value

  return {
    count: items.length,
    getScrollElement: () => scroller.value,
    getItemKey: (index: number) => items[index]?.path ?? index,
    estimateSize: () => ROW_HEIGHT,
    scrollMargin: listOffset.value,
    overscan: 6,
  }
}))

const rows = computed(() => virtualizer.value.getVirtualItems().flatMap((item) => {
  const mod = visible.value[item.index]
  return mod ? [{ item, mod }] : []
}))

function measure(el: Element | ComponentPublicInstance | null) {
  if (el instanceof Element) virtualizer.value.measureElement(el)
}

function measureOffset() {
  if (!list.value || !scroller.value) return

  listOffset.value = list.value.getBoundingClientRect().top - scroller.value.getBoundingClientRect().top + scroller.value.scrollTop
}

useResizeObserver(() => scroller.value?.firstElementChild as HTMLElement | null | undefined, measureOffset)

const picked = computed(() => mods.value.filter(mod => selected.value.includes(modKey(mod))))
const pickedManaged = computed(() => picked.value.filter(mod => mod.managed))

const allVisiblePicked = computed(() =>
  visible.value.length > 0 && visible.value.every(mod => selected.value.includes(modKey(mod))),
)

function loaders(mod: ModFile) {
  return mod.details.loaders.map(loader => MOD_LOADER_LABELS[loader] ?? loader)
}

function subtitle(mod: ModFile) {
  return [mod.details.modId, mod.details.version].filter(part => part.trim()).join(' · ')
}

function toggle(path: string) {
  expanded.value = expanded.value === path ? '' : path
}

function togglePicked(path: string) {
  selected.value = selected.value.includes(path)
    ? selected.value.filter(item => item !== path)
    : [...selected.value, path]
}

function toggleAllVisible() {
  const keys = visible.value.map(modKey)

  selected.value = allVisiblePicked.value
    ? selected.value.filter(key => !keys.includes(key))
    : [...new Set([...selected.value, ...keys])]
}

function keepAlive() {
  const keys = mods.value.map(modKey)

  selected.value = selected.value.filter(key => keys.includes(key))
  if (!keys.includes(expanded.value)) expanded.value = ''
}

async function load(force = false) {
  const result = await attempt(
    () => modsStore.load(instanceId.value, force),
    { context: { instanceId: instanceId.value, action: t('instance.mods.list_action') } },
  )

  if (!result.ok) return

  keepAlive()
  await identify()
}

async function identify() {
  if (!mods.value.length) return

  await attempt(
    () => modsStore.identify(instanceId.value),
    { context: { instanceId: instanceId.value, action: t('instance.mods.identify_action') } },
  )
}

async function checkUpdates() {
  const result = await attempt(
    () => modsStore.checkUpdates(instanceId.value),
    { context: { instanceId: instanceId.value, action: t('instance.mods.check_action') } },
  )

  if (!result.ok) return

  toast.add({
    title: result.value.length
      ? t('instance.mods.updates_count', { count: result.value.length })
      : t('instance.mods.up_to_date'),
    color: result.value.length ? 'success' : 'neutral',
    icon: 'i-lucide-arrow-up-circle',
  })
}

async function applyUpdates(paths: string[]) {
  if (!paths.length || working.value) return

  working.value = true

  const result = await attempt(
    () => modsStore.update(instanceId.value, paths),
    { context: { instanceId: instanceId.value, action: t('instance.mods.update_action') } },
  )

  working.value = false

  if (!result.ok) return

  const { updated, failed } = result.value

  toast.add({
    title: updated.length ? t('instance.mods.updated_count', { count: updated.length }) : t('instance.mods.nothing_updated'),
    description: failed.length ? t('instance.mods.update_failed', { list: failed.join(', ') }) : undefined,
    color: failed.length ? 'warning' : 'success',
    icon: 'i-lucide-arrow-up-circle',
  })

  keepAlive()
  await identify()
}

async function setEnabled(mod: ModFile, enabled: boolean) {
  if (busy.value.includes(mod.path)) return

  busy.value = [...busy.value, mod.path]

  const result = await attempt(
    () => modsStore.setEnabled(instanceId.value, mod.path, enabled),
    { context: { instanceId: instanceId.value, action: enabled ? t('instance.mods.enable_action') : t('instance.mods.disable_action') } },
  )

  busy.value = busy.value.filter(path => path !== mod.path)

  if (!result.ok) return

  keepAlive()
}

async function setPickedEnabled(enabled: boolean) {
  const targets = picked.value.filter(mod => mod.enabled !== enabled)
  if (!targets.length || working.value) return

  working.value = true

  for (const mod of targets) {
    const result = await attempt(
      () => modsStore.setEnabled(instanceId.value, mod.path, enabled),
      { context: { instanceId: instanceId.value, action: enabled ? t('instance.mods.enable_action') : t('instance.mods.disable_action') } },
    )

    if (!result.ok) break
  }

  working.value = false

  keepAlive()
}

async function removePicked() {
  const paths = picked.value.map(mod => mod.path)
  if (!paths.length) return

  working.value = true

  const result = await attempt(
    () => modsStore.remove(instanceId.value, paths),
    { context: { instanceId: instanceId.value, action: t('instance.mods.remove_action') } },
  )

  working.value = false
  confirming.value = false

  if (!result.ok) return

  selected.value = []
  keepAlive()

  toast.add({
    title: paths.length === 1 ? t('instance.mods.removed_one') : t('instance.mods.removed_many', { count: paths.length }),
    color: 'success',
    icon: 'i-lucide-trash-2',
  })
}

function report(installed: InstalledMods) {
  const parts = [
    installed.added.length ? t('instance.mods.report.added', { count: installed.added.length }) : '',
    installed.replaced.length ? t('instance.mods.report.replaced', { count: installed.replaced.length }) : '',
    installed.skipped.length ? t('instance.mods.report.skipped', { count: installed.skipped.length }) : '',
    installed.failed.length ? t('instance.mods.report.failed', { count: installed.failed.length }) : '',
  ].filter(Boolean)

  const aside = [...installed.skipped, ...installed.failed]

  toast.add({
    title: parts.length ? parts.join(', ') : t('instance.mods.report.nothing'),
    description: aside.length ? aside.join(', ') : undefined,
    color: installed.failed.length ? 'error' : installed.added.length || installed.replaced.length ? 'success' : 'warning',
    icon: 'i-lucide-package-plus',
  })
}

async function addFiles(paths: string[]) {
  if (!paths.length || working.value) return

  working.value = true

  const result = await attempt(
    () => modsStore.add(instanceId.value, paths),
    { context: { instanceId: instanceId.value, action: t('instance.mods.add_action') } },
  )

  working.value = false

  if (!result.ok) return

  report(result.value)
  keepAlive()
}

async function afterCatalogInstall() {
  keepAlive()
  await identify()
}

async function pickFiles() {
  const result = await attempt(
    () => call('pick_mod_files', { dialog: { title: t('dialog.mod_files.title'), filter: t('dialog.mod_files.filter') } }),
    { context: { instanceId: instanceId.value, action: t('instance.mods.pick_action') } },
  )

  if (result.ok) await addFiles(result.value)
}

const openFolder = () => safeRun(
  () => call('open_instance_dir', { instanceId: instanceId.value, target: 'mods' }),
  { context: { instanceId: instanceId.value, action: t('instance.mods.open_folder_action') } },
)

const openHomepage = (url: string) => safeRun(
  () => call('open_url', { url }),
  { context: { instanceId: instanceId.value, action: t('instance.mods.open_page_action') } },
)

onMounted(() => {
  scroller.value = root.value?.closest<HTMLElement>('[data-main-scroll]') ?? null
  load()
})

const dragging = useFileDrop(async (paths) => {
  if (loading.value) return

  const files = paths.filter(path => isModFile(path))

  if (!files.length) {
    toast.add({ title: t('instance.mods.not_mods'), description: t('instance.mods.not_mods_hint'), color: 'warning' })
    return
  }

  await addFiles(files)
})

watch(instanceId, () => {
  expanded.value = ''
  query.value = ''
  selected.value = []
  load()
})
</script>

<template>
  <div
    ref="root"
    class="flex flex-col gap-3"
  >
    <div class="flex items-center gap-2">
      <KitSearchInput
        v-model="query"
        :placeholder="$t('instance.mods.search_placeholder')"
        :aria-label="$t('instance.mods.search')"
        class="w-65"
      />

      <RadioGroup
        v-model="filter"
        :aria-label="$t('instance.mods.show')"
        class="w-fit"
      >
        <RadioGroupItem
          v-for="item in FILTERS"
          :key="item.value"
          :value="item.value"
          class="flex-none"
        >
          {{ item.label }}
          <span
            v-if="item.count"
            class="text-fg-faint tabular-nums"
          >{{ item.count }}</span>
        </RadioGroupItem>
      </RadioGroup>

      <span class="flex-1" />

      <Button
        icon="i-lucide-compass"
        @click="catalogOpen = true"
      >
        {{ $t('instance.mods.find') }}
      </Button>

      <Button
        icon="i-lucide-package-plus"
        :loading="working"
        @click="pickFiles"
      >
        {{ $t('instance.mods.add_files') }}
      </Button>

      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button
            size="icon"
            icon="i-lucide-ellipsis"
            :aria-label="$t('instance.panel.more')"
            :title="$t('instance.panel.more')"
          />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem
            icon="i-lucide-arrow-up-circle"
            :disabled="!mods.length || checking"
            @select="checkUpdates"
          >
            {{ $t('instance.mods.check_updates') }}
          </DropdownMenuItem>
          <DropdownMenuItem
            icon="i-lucide-refresh-cw"
            :disabled="loading"
            @select="load(true)"
          >
            {{ $t('instance.mods.reload') }}
          </DropdownMenuItem>
          <DropdownMenuItem
            icon="i-lucide-folder-open"
            @select="openFolder"
          >
            {{ $t('instance.mods.folder') }}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>

    <Alert
      v-if="updates.length"
      variant="accent"
      icon="i-lucide-arrow-up-circle"
      class="items-center py-2 pr-2"
    >
      <span class="font-medium text-fg">{{ $t('instance.mods.updates_count', { count: updates.length }) }}</span>

      <template #action>
        <div class="flex items-center gap-1.5">
          <Button
            v-if="filter !== 'outdated'"
            variant="ghost"
            size="sm"
            @click="filter = 'outdated'"
          >
            {{ $t('instance.mods.show_updates') }}
          </Button>
          <Button
            size="sm"
            :loading="working"
            @click="applyUpdates(updates.map(update => update.path))"
          >
            {{ $t('instance.mods.update_all') }}
          </Button>
        </div>
      </template>
    </Alert>

    <div
      class="overflow-hidden rounded-md bg-panel ring-inset transition-shadow duration-150"
      :class="dragging ? 'ring-2 ring-accent' : 'ring-1 ring-card-line'"
    >
      <div
        class="grid h-10 items-center gap-x-3.5 border-b border-line pr-3 pl-3.5 text-caption text-fg-faint"
        :class="GRID"
      >
        <Checkbox
          :model-value="allVisiblePicked"
          :disabled="!visible.length"
          :aria-label="$t('instance.mods.select_all')"
          @update:model-value="toggleAllVisible"
        />
        <span class="col-span-2 flex min-w-0 items-center gap-3">
          <span class="truncate">
            <template v-if="visible.length !== mods.length">
              {{ $t('instance.mods.count_of', { visible: visible.length, total: mods.length }) }}
            </template>
            <template v-else>
              {{ $t('instance.mods.count', { count: visible.length }) }}
            </template>
            <template v-if="disabledCount">
              · {{ $t('instance.mods.disabled_count', { count: disabledCount }) }}
            </template>
          </span>

          <KitStatus
            v-if="loading || dragging || identifying || checking"
            :tone="dragging ? 'accent' : 'muted'"
            dot="live"
            class="shrink-0"
          >
            <template v-if="dragging">
              {{ $t('instance.mods.drop_here') }}
            </template>
            <template v-else-if="loading">
              {{ $t('instance.mods.reading') }}
            </template>
            <template v-else-if="checking">
              {{ $t('instance.mods.asking') }}
            </template>
            <template v-else>
              {{ $t('instance.mods.identifying') }}
            </template>
          </KitStatus>
        </span>
        <span>{{ $t('instance.mods.col.version') }}</span>
        <span>{{ $t('instance.mods.col.source') }}</span>
        <span />
        <span />
      </div>

      <div
        v-if="visible.length"
        ref="list"
        class="relative"
        :style="{ height: `${virtualizer.getTotalSize()}px` }"
      >
        <div
          v-for="{ item, mod } in rows"
          :key="mod.path"
          :ref="measure"
          :data-index="item.index"
          class="absolute inset-x-0 top-0"
          :class="item.index < visible.length - 1 && 'border-b border-line'"
          :style="{ transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
        >
          <div
            class="group/row grid min-h-14 items-center gap-x-3.5 py-2 pr-3 pl-3.5 transition-colors duration-150"
            :class="[GRID, selected.includes(modKey(mod)) ? 'bg-accent/10' : 'hover:bg-hover']"
          >
            <Checkbox
              :model-value="selected.includes(modKey(mod))"
              :aria-label="modName(mod, matchOf(mod.path))"
              @update:model-value="togglePicked(modKey(mod))"
            />

            <InstanceModIcon
              :icon-key="mod.details.iconKey"
              :fallback-url="matchOf(mod.path)?.iconUrl"
              :name="modName(mod, matchOf(mod.path))"
              :class="!mod.enabled && 'opacity-45'"
            />

            <button
              type="button"
              class="min-w-0 cursor-pointer rounded-xs text-left outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
              :class="!mod.enabled && 'opacity-45'"
              :aria-expanded="expanded === modKey(mod)"
              @click="toggle(modKey(mod))"
            >
              <span class="flex min-w-0 items-center gap-2">
                <span
                  class="truncate font-medium text-fg"
                  :title="mod.fileName"
                >
                  {{ modName(mod, matchOf(mod.path)) }}
                </span>
                <Badge
                  v-if="mod.managed"
                  :title="$t('instance.mods.managed_hint')"
                >
                  {{ $t('instance.mods.from_pack') }}
                </Badge>
              </span>
              <span class="mt-0.5 block truncate text-caption text-fg-faint">
                {{ mod.details.description || subtitle(mod) || mod.fileName }}
              </span>
            </button>

            <span
              class="truncate text-caption text-fg-muted tabular-nums"
              :class="!mod.enabled && 'opacity-45'"
            >
              {{ mod.details.version || '-' }}
              <span
                v-if="updateOf(mod.path)"
                class="text-accent"
              >→ {{ updateOf(mod.path)?.to }}</span>
            </span>

            <span
              class="flex min-w-0 items-center gap-1.5 text-caption text-fg-muted"
              :class="!mod.enabled && 'opacity-45'"
            >
              <template v-if="matchOf(mod.path)">
                <img
                  :src="PROVIDER_LOGOS[matchOf(mod.path)!.provider]"
                  alt=""
                  class="size-3.5 shrink-0"
                >
                <span class="truncate">{{ CATALOG_LABELS[matchOf(mod.path)!.provider] }}</span>
              </template>
              <template v-else-if="!identifying">
                <Icon
                  name="i-lucide-file-archive"
                  class="size-3.5 shrink-0"
                />
                <span class="truncate">{{ $t('instance.mods.source_file') }}</span>
              </template>
            </span>

            <Switch
              :model-value="mod.enabled"
              :disabled="busy.includes(mod.path) || working"
              :aria-label="modName(mod, matchOf(mod.path))"
              @update:model-value="value => setEnabled(mod, value)"
            />

            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  icon="i-lucide-ellipsis"
                  :aria-label="$t('instance.mods.actions', { name: modName(mod, matchOf(mod.path)) })"
                  class="opacity-0 group-hover/row:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
                />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  icon="i-lucide-info"
                  @select="toggle(modKey(mod))"
                >
                  {{ $t('common.details') }}
                </DropdownMenuItem>
                <DropdownMenuItem
                  v-if="updateOf(mod.path) && !mod.managed"
                  icon="i-lucide-arrow-up-circle"
                  :disabled="working"
                  @select="applyUpdates([mod.path])"
                >
                  {{ $t('instance.mods.update_to', { version: updateOf(mod.path)?.to }) }}
                </DropdownMenuItem>
                <DropdownMenuItem
                  v-if="matchOf(mod.path)?.pageUrl"
                  icon="i-lucide-external-link"
                  @select="openHomepage(matchOf(mod.path)!.pageUrl)"
                >
                  {{ CATALOG_LABELS[matchOf(mod.path)!.provider] }}
                </DropdownMenuItem>
                <DropdownMenuItem
                  v-if="mod.details.homepage"
                  icon="i-lucide-globe"
                  @select="openHomepage(mod.details.homepage)"
                >
                  {{ $t('instance.mods.page') }}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  icon="i-lucide-trash-2"
                  variant="danger"
                  :disabled="working"
                  @select="askRemove(mod)"
                >
                  {{ $t('common.delete') }}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          <div
            v-if="expanded === modKey(mod)"
            class="animate-rise space-y-3 pr-3 pb-4 pl-23.5"
          >
            <p
              v-if="mod.details.description"
              class="text-body text-fg-muted"
            >
              {{ mod.details.description }}
            </p>

            <KitDetails>
              <KitDetail
                v-if="modAuthors(mod)"
                :label="$t('instance.mods.authors')"
              >
                {{ modAuthors(mod) }}
              </KitDetail>

              <KitDetail
                v-if="mod.details.license"
                :label="$t('instance.mods.license')"
              >
                {{ mod.details.license }}
              </KitDetail>

              <KitDetail
                :label="$t('instance.mods.file')"
                :title="mod.fileName"
                mono
              >
                {{ mod.fileName }}<template v-if="modSize(mod.size)">
                  · {{ modSize(mod.size) }}
                </template>
              </KitDetail>

              <KitDetail
                v-if="mod.details.homepage"
                :label="$t('instance.mods.page')"
              >
                <KitLink @click="openHomepage(mod.details.homepage)">
                  {{ mod.details.homepage }}
                </KitLink>
              </KitDetail>
            </KitDetails>

            <div
              v-if="loaders(mod).length"
              class="flex flex-wrap gap-1.5"
            >
              <Badge
                v-for="loader in loaders(mod)"
                :key="loader"
              >
                {{ loader }}
              </Badge>
            </div>

            <KitNote
              v-if="mod.managed"
              icon="i-lucide-package"
              tone="muted"
            >
              {{ $t('instance.mods.managed_hint') }}
            </KitNote>
          </div>
        </div>
      </div>

      <p
        v-else
        class="px-4 py-12 text-center text-body text-fg-faint"
      >
        <template v-if="loading || !loaded">
          {{ $t('instance.mods.reading_folder') }}
        </template>
        <template v-else-if="mods.length">
          {{ $t('instance.mods.nothing_found') }}
        </template>
        <template v-else-if="instance.type === 'vanilla'">
          {{ $t('instance.mods.no_loader') }}
        </template>
        <template v-else>
          {{ $t('instance.mods.empty') }}
        </template>
      </p>
    </div>

    <KitFloatingBar
      :open="picked.length > 0 && !confirming"
      :label="$t('instance.mods.picked', { count: picked.length })"
      position="center"
    >
      <Button
        variant="ghost"
        size="sm"
        icon="i-lucide-toggle-right"
        :disabled="working"
        @click="setPickedEnabled(true)"
      >
        {{ $t('instance.mods.enable') }}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        icon="i-lucide-toggle-left"
        :disabled="working"
        @click="setPickedEnabled(false)"
      >
        {{ $t('instance.mods.disable') }}
      </Button>
      <Button
        variant="danger-ghost"
        size="sm"
        icon="i-lucide-trash-2"
        :disabled="working"
        @click="confirming = true"
      >
        {{ $t('common.delete') }}
      </Button>
      <Button
        size="sm"
        @click="selected = []"
      >
        {{ $t('instance.mods.unpick') }}
      </Button>
    </KitFloatingBar>

    <InstanceModCatalog
      v-model:open="catalogOpen"
      :instance="instance"
      @installed="afterCatalogInstall"
    />

    <KitConfirmDialog
      v-model:open="confirming"
      :title="$t('instance.mods.remove_title')"
      :description="$t('instance.mods.remove_hint')"
      :confirm-label="$t('instance.mods.remove_confirm', { count: picked.length })"
      :loading="working"
      @confirm="removePicked"
    >
      <ul class="max-h-52 space-y-1 overflow-auto rounded-sm bg-sheet p-3">
        <li
          v-for="mod in picked"
          :key="mod.path"
          class="truncate font-mono text-caption text-fg-muted"
        >
          {{ mod.fileName }}
        </li>
      </ul>

      <KitStatus
        v-if="pickedManaged.length"
        tone="warning"
      >
        {{ $t('instance.mods.remove_managed', { count: pickedManaged.length }) }}
      </KitStatus>
    </KitConfirmDialog>
  </div>
</template>
