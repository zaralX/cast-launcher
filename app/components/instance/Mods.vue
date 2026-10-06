<script setup lang="ts">
import type { Instance } from '~/types/instance'
import type { CatalogMatch, InstalledMods, ModFile, ModUpdate } from '~/types/mods'
import { CATALOG_LABELS, MOD_LOADER_LABELS } from '~/types/mods'

const props = defineProps<{ instance: Instance }>()

const ICON_BATCH = 10

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

const FILTERS = computed(() => [
  { label: t('instance.mods.filter.all'), value: 'all' },
  { label: t('instance.mods.filter.enabled'), value: 'enabled' },
  { label: t('instance.mods.filter.disabled'), value: 'disabled' },
  { label: t('instance.mods.filter.outdated'), value: 'outdated' },
])

const identifying = computed(() => modsStore.isIdentifying(instanceId.value))
const checking = computed(() => modsStore.isChecking(instanceId.value))

const updates = computed(() => modsStore.updatesOf(instanceId.value))

const matchOf = (path: string): CatalogMatch | null => modsStore.matchOf(instanceId.value, path)
const updateOf = (path: string): ModUpdate | null => updates.value.find(update => update.path === path) ?? null

const disabledCount = computed(() => mods.value.filter(mod => !mod.enabled).length)

const visible = computed(() => mods.value.filter((mod) => {
  if (filter.value === 'enabled' && !mod.enabled) return false
  if (filter.value === 'disabled' && mod.enabled) return false
  if (filter.value === 'outdated' && !updateOf(mod.path)) return false

  return matchesMod(mod, query.value, matchOf(mod.path))
}))

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

async function loadIcons(list: ModFile[]) {
  const keys = list.map(mod => mod.details.iconKey).filter((key): key is string => !!key)

  for (let index = 0; index < keys.length; index += ICON_BATCH) {
    await Promise.all(keys.slice(index, index + ICON_BATCH).map(key => modsStore.ensureIcon(key)))
  }
}

async function load(force = false) {
  const result = await attempt(
    () => modsStore.load(instanceId.value, force),
    { context: { instanceId: instanceId.value, action: t('instance.mods.list_action') } },
  )

  if (!result.ok) return

  keepAlive()
  await loadIcons(result.value)
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
  await loadIcons(mods.value)
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
  await loadIcons(result.value)
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
  await loadIcons(mods.value)
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
  await loadIcons(mods.value)
}

async function afterCatalogInstall() {
  keepAlive()
  await loadIcons(mods.value)
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

onMounted(load)

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
  <KitPanel
    index="01"
    :title="$t('instance.mods.title')"
    icon="i-lucide-blocks"
  >
    <div
      class="space-y-5"
      :class="dragging ? 'outline outline-1 outline-offset-8 outline-acid' : ''"
    >
      <div class="flex flex-wrap items-end gap-4">
        <KitField
          :label="$t('instance.mods.search')"
          class="min-w-56 flex-1"
        >
          <KitSearchInput
            v-model="query"
            :placeholder="$t('instance.mods.search_placeholder')"
          />
        </KitField>

        <KitField
          :label="$t('instance.mods.show')"
          class="min-w-40"
        >
          <KitSelect
            v-model="filter"
            :items="FILTERS"
          />
        </KitField>

        <div class="flex items-center gap-3">
          <Button
            icon="i-lucide-search"
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

          <Button
            icon="i-lucide-arrow-up-circle"
            :loading="checking"
            :disabled="!mods.length"
            @click="checkUpdates"
          >
            {{ $t('instance.mods.check_updates') }}
          </Button>

          <Button
            icon="i-lucide-refresh-cw"
            :loading="loading"
            @click="load(true)"
          >
            {{ $t('instance.mods.reload') }}
          </Button>

          <Button
            icon="i-lucide-folder-open"
            @click="openFolder"
          >
            {{ $t('instance.mods.folder') }}
          </Button>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-between gap-4 border-t border-line pt-4">
        <div class="flex items-center gap-4">
          <Label
            v-if="visible.length"
            class="flex cursor-pointer items-center gap-2.5"
          >
            <Checkbox
              :model-value="allVisiblePicked"
              @update:model-value="toggleAllVisible"
            />
            {{ $t('instance.mods.select_all') }}
          </Label>

          <p class="font-mono text-label uppercase tracking-caps text-fg-faint">
            <template v-if="visible.length !== mods.length">
              {{ $t('instance.mods.count_of', { visible: visible.length, total: mods.length }) }}
            </template>
            <template v-else>
              {{ $t('instance.mods.count', { count: visible.length }) }}
            </template>
            <template v-if="disabledCount">
              · {{ $t('instance.mods.disabled_count', { count: disabledCount }) }}
            </template>
          </p>
        </div>

        <KitStatus
          v-if="loading || dragging || identifying || checking"
          :tone="dragging ? 'accent' : 'muted'"
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
      </div>

      <Alert
        v-if="updates.length"
        variant="accent"
        class="items-center"
      >
        <p class="font-mono text-label uppercase tracking-caps text-fg-muted">
          {{ $t('instance.mods.updates_count', { count: updates.length }) }}
        </p>

        <template #action>
          <Button
            icon="i-lucide-arrow-up-circle"
            :loading="working"
            @click="applyUpdates(updates.map(update => update.path))"
          >
            {{ $t('instance.mods.update_all') }}
          </Button>
        </template>
      </Alert>

      <Alert
        v-if="picked.length"
        class="items-center"
      >
        <p class="font-mono text-label uppercase tracking-caps text-fg-muted">
          {{ $t('instance.mods.picked', { count: picked.length }) }}
        </p>

        <template #action>
          <div class="flex items-center gap-4">
            <Button
              variant="quiet"
              icon="i-lucide-toggle-right"
              :disabled="working"
              @click="setPickedEnabled(true)"
            >
              {{ $t('instance.mods.enable') }}
            </Button>

            <Button
              variant="quiet"
              icon="i-lucide-toggle-left"
              :disabled="working"
              @click="setPickedEnabled(false)"
            >
              {{ $t('instance.mods.disable') }}
            </Button>

            <Button
              variant="quiet-danger"
              icon="i-lucide-trash-2"
              :disabled="working"
              @click="confirming = true"
            >
              {{ $t('common.delete') }}
            </Button>

            <Button
              variant="quiet"
              icon="i-lucide-x"
              @click="selected = []"
            >
              {{ $t('instance.mods.unpick') }}
            </Button>
          </div>
        </template>
      </Alert>

      <div
        v-if="visible.length"
        class="border-t border-line"
      >
        <div
          v-for="mod in visible"
          :key="mod.path"
          class="border-b border-line"
        >
          <div
            class="group flex items-center gap-4 px-1 py-3 transition-colors duration-300 hover:bg-ink-700"
            :class="mod.enabled ? '' : 'opacity-55'"
          >
            <Checkbox
              :model-value="selected.includes(modKey(mod))"
              @update:model-value="togglePicked(modKey(mod))"
            />

            <Switch
              :model-value="mod.enabled"
              :disabled="busy.includes(mod.path) || working"
              @update:model-value="value => setEnabled(mod, value)"
            />

            <button
              type="button"
              class="flex min-w-0 flex-1 cursor-pointer items-center gap-4 text-left outline-none"
              @click="toggle(modKey(mod))"
            >
              <InstanceModIcon
                :icon-key="mod.details.iconKey"
                :fallback-url="matchOf(mod.path)?.iconUrl"
                :name="modName(mod, matchOf(mod.path))"
              />

              <span class="min-w-0 flex-1">
                <span class="flex items-center gap-2.5">
                  <span
                    class="truncate text-title font-medium text-fg"
                    :title="mod.fileName"
                  >
                    {{ modName(mod, matchOf(mod.path)) }}
                  </span>

                  <Badge
                    v-if="updateOf(mod.path)"
                    variant="accent"
                  >
                    {{ updateOf(mod.path)?.to }}
                  </Badge>

                  <Icon
                    v-if="mod.managed"
                    name="i-lucide-package"
                    class="size-3.5 shrink-0 text-fg-faint"
                    :title="$t('instance.mods.managed')"
                  />
                </span>

                <span class="mt-1 block truncate font-mono text-label uppercase tracking-caps text-fg-faint">
                  {{ subtitle(mod) || mod.fileName }}
                </span>
              </span>

              <span class="flex shrink-0 items-center gap-2">
                <Badge
                  v-for="loader in loaders(mod)"
                  :key="loader"
                >
                  {{ loader }}
                </Badge>
              </span>

              <Icon
                name="i-lucide-chevron-down"
                class="size-4 shrink-0 text-fg-faint transition-transform duration-300"
                :class="expanded === modKey(mod) ? 'rotate-180 text-acid' : ''"
              />
            </button>
          </div>

          <div
            v-if="expanded === modKey(mod)"
            class="space-y-3 px-1 pb-5 pl-[6.5rem] animate-rise"
          >
            <p
              v-if="mod.details.description"
              class="text-body leading-relaxed text-fg-muted"
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
              v-if="matchOf(mod.path)"
              class="flex flex-wrap items-center gap-4"
            >
              <Button
                v-if="matchOf(mod.path)?.pageUrl"
                variant="link"
                icon="i-lucide-external-link"
                @click="openHomepage(matchOf(mod.path)!.pageUrl)"
              >
                {{ CATALOG_LABELS[matchOf(mod.path)!.provider] }}
              </Button>

              <Button
                v-if="updateOf(mod.path) && !mod.managed"
                size="sm"
                icon="i-lucide-arrow-up-circle"
                :loading="working"
                @click="applyUpdates([mod.path])"
              >
                {{ $t('instance.mods.update_to', { version: updateOf(mod.path)?.to }) }}
              </Button>
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

      <KitStatus
        v-else
        class="border-t border-line py-10"
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
      </KitStatus>
    </div>

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
      <ul class="max-h-52 space-y-1.5 overflow-auto border border-line bg-ink-900 p-4">
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
  </KitPanel>
</template>
