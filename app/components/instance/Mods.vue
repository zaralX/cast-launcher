<script setup lang="ts">
import {getCurrentWebview} from "@tauri-apps/api/webview"
import type {UnlistenFn} from "@tauri-apps/api/event"
import type {Instance} from "~/types/instance"
import type {CatalogMatch, InstalledMods, ModFile, ModUpdate} from "~/types/mods"
import {
  CATALOG_LABELS,
  MOD_LOADER_LABELS,
  isModFile,
  matchesMod,
  modAuthors,
  modKey,
  modName,
  modSize
} from "~/types/mods"
import {call} from "~/types/backend"

const props = defineProps<{ instance: Instance }>()

const ICON_BATCH = 10

type Filter = "all" | "enabled" | "disabled" | "outdated"

const modsStore = useModsStore()
const toast = useToast()

const instanceId = computed(() => props.instance.id)

const mods = computed(() => modsStore.listOf(instanceId.value))
const loading = computed(() => modsStore.isLoading(instanceId.value))
const loaded = computed(() => modsStore.isLoaded(instanceId.value))

const query = ref("")
const filter = ref<Filter>("all")
const catalogOpen = ref(false)
const expanded = ref("")
const selected = ref<string[]>([])
const busy = ref<string[]>([])
const working = ref(false)
const confirming = ref(false)
const dragging = ref(false)

const FILTERS = [
  {label: "Все", value: "all"},
  {label: "Включённые", value: "enabled"},
  {label: "Выключенные", value: "disabled"},
  {label: "С обновлениями", value: "outdated"}
]

const identifying = computed(() => modsStore.isIdentifying(instanceId.value))
const checking = computed(() => modsStore.isChecking(instanceId.value))

const updates = computed(() => modsStore.updatesOf(instanceId.value))

const matchOf = (path: string): CatalogMatch | null => modsStore.matchOf(instanceId.value, path)
const updateOf = (path: string): ModUpdate | null => updates.value.find(update => update.path === path) ?? null

const disabledCount = computed(() => mods.value.filter(mod => !mod.enabled).length)

const visible = computed(() => mods.value.filter(mod => {
  if (filter.value === "enabled" && !mod.enabled) return false
  if (filter.value === "disabled" && mod.enabled) return false
  if (filter.value === "outdated" && !updateOf(mod.path)) return false

  return matchesMod(mod, query.value, matchOf(mod.path))
}))

const picked = computed(() => mods.value.filter(mod => selected.value.includes(modKey(mod))))
const pickedManaged = computed(() => picked.value.filter(mod => mod.managed))

const allVisiblePicked = computed(() =>
    visible.value.length > 0 && visible.value.every(mod => selected.value.includes(modKey(mod)))
)

function loaders(mod: ModFile) {
  return mod.details.loaders.map(loader => MOD_LOADER_LABELS[loader] ?? loader)
}

function subtitle(mod: ModFile) {
  return [mod.details.modId, mod.details.version].filter(part => part.trim()).join(" · ")
}

function toggle(path: string) {
  expanded.value = expanded.value === path ? "" : path
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
  if (!keys.includes(expanded.value)) expanded.value = ""
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
      {context: {instanceId: instanceId.value, action: "Список модов"}}
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
      {context: {instanceId: instanceId.value, action: "Опознание модов"}}
  )
}

async function checkUpdates() {
  const result = await attempt(
      () => modsStore.checkUpdates(instanceId.value),
      {context: {instanceId: instanceId.value, action: "Проверка обновлений модов"}}
  )

  if (!result.ok) return

  toast.add({
    title: result.value.length
        ? `Обновлений: ${result.value.length}`
        : "Все моды свежие",
    color: result.value.length ? "success" : "neutral",
    icon: "i-lucide-arrow-up-circle"
  })
}

async function applyUpdates(paths: string[]) {
  if (!paths.length || working.value) return

  working.value = true

  const result = await attempt(
      () => modsStore.update(instanceId.value, paths),
      {context: {instanceId: instanceId.value, action: "Обновление модов"}}
  )

  working.value = false

  if (!result.ok) return

  const {updated, failed} = result.value

  toast.add({
    title: updated.length ? `Обновлено: ${updated.length}` : "Ничего не обновилось",
    description: failed.length ? `Не удалось: ${failed.join(", ")}` : undefined,
    color: failed.length ? "warning" : "success",
    icon: "i-lucide-arrow-up-circle"
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
      {context: {instanceId: instanceId.value, action: enabled ? "Включение мода" : "Выключение мода"}}
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
        {context: {instanceId: instanceId.value, action: enabled ? "Включение мода" : "Выключение мода"}}
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
      {context: {instanceId: instanceId.value, action: "Удаление модов"}}
  )

  working.value = false
  confirming.value = false

  if (!result.ok) return

  selected.value = []
  keepAlive()

  toast.add({
    title: paths.length === 1 ? "Мод удалён" : `Удалено модов: ${paths.length}`,
    color: "success",
    icon: "i-lucide-trash-2"
  })
}

function report(installed: InstalledMods) {
  const parts = [
    installed.added.length ? `добавлено ${installed.added.length}` : "",
    installed.replaced.length ? `заменено ${installed.replaced.length}` : "",
    installed.skipped.length ? `пропущено ${installed.skipped.length}` : "",
    installed.failed.length ? `не удалось ${installed.failed.length}` : ""
  ].filter(Boolean)

  const aside = [...installed.skipped, ...installed.failed]

  toast.add({
    title: parts.length ? parts.join(", ") : "Ничего не добавлено",
    description: aside.length ? aside.join(", ") : undefined,
    color: installed.failed.length ? "error" : installed.added.length || installed.replaced.length ? "success" : "warning",
    icon: "i-lucide-package-plus"
  })
}

async function addFiles(paths: string[]) {
  if (!paths.length || working.value) return

  working.value = true

  const result = await attempt(
      () => modsStore.add(instanceId.value, paths),
      {context: {instanceId: instanceId.value, action: "Добавление модов"}}
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
      () => call("pick_mod_files"),
      {context: {instanceId: instanceId.value, action: "Выбор файлов модов"}}
  )

  if (result.ok) await addFiles(result.value)
}

const openFolder = () => safeRun(
    () => call("open_instance_dir", {instanceId: instanceId.value, target: "mods"}),
    {context: {instanceId: instanceId.value, action: "Открытие папки модов"}}
)

const openHomepage = (url: string) => safeRun(
    () => call("open_url", {url}),
    {context: {instanceId: instanceId.value, action: "Открытие страницы мода"}}
)

let unlisten: UnlistenFn | null = null

onMounted(async () => {
  await load()

  unlisten = await getCurrentWebview().onDragDropEvent(async ({payload}) => {
    if (payload.type === "enter" || payload.type === "over") {
      dragging.value = true
      return
    }

    dragging.value = false

    if (payload.type !== "drop") return

    const files = payload.paths.filter(path => isModFile(path))

    if (!files.length) {
      toast.add({title: "Это не моды", description: "Перетащите jar, zip или litemod", color: "warning"})
      return
    }

    await addFiles(files)
  })
})

onBeforeUnmount(() => {
  unlisten?.()
  unlisten = null
})

watch(instanceId, () => {
  expanded.value = ""
  query.value = ""
  selected.value = []
  load()
})
</script>

<template>
  <SettingsPanel
      index="01"
      title="Моды"
      icon="i-lucide-blocks"
      description="Содержимое папки mods этой сборки: то, что лежит на диске, а не то, что заявлено в модпаке."
  >
    <div
        class="space-y-5"
        :class="dragging ? 'outline outline-1 outline-acid outline-offset-8' : ''"
    >
      <div class="flex flex-wrap items-end gap-4">
        <SettingsField label="Поиск" class="min-w-[14rem] flex-1">
          <UInput v-model="query" placeholder="Название, modId или файл" class="w-full">
            <template #trailing>
              <UIcon name="i-lucide-search" class="size-3.5 text-fg-faint"/>
            </template>
          </UInput>
        </SettingsField>

        <SettingsField label="Показывать" class="min-w-[10rem]">
          <USelect v-model="filter" :items="FILTERS" class="w-full"/>
        </SettingsField>

        <div class="flex items-center gap-3 pb-1">
          <AppButton
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              icon="i-lucide-search"
              @click="catalogOpen = true"
          >
            Найти мод
          </AppButton>

          <AppButton
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              icon="i-lucide-package-plus"
              :loading="working"
              @click="pickFiles"
          >
            Добавить файлы
          </AppButton>

          <AppButton
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              icon="i-lucide-arrow-up-circle"
              :loading="checking"
              :disabled="!mods.length"
              @click="checkUpdates"
          >
            Проверить обновления
          </AppButton>

          <AppButton
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              icon="i-lucide-refresh-cw"
              :loading="loading"
              @click="load(true)"
          >
            Перечитать
          </AppButton>

          <AppButton
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              icon="i-lucide-folder-open"
              @click="openFolder"
          >
            Папка
          </AppButton>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-between gap-4 border-t border-line pt-4">
        <div class="flex items-center gap-4">
          <label v-if="visible.length" class="flex cursor-pointer items-center gap-2.5">
            <UCheckbox :model-value="allVisiblePicked" @update:model-value="toggleAllVisible"/>
            <span class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">Выбрать все</span>
          </label>

          <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
            {{ visible.length }}<template v-if="visible.length !== mods.length"> из {{ mods.length }}</template>
            модов<template v-if="disabledCount"> · {{ disabledCount }} выключено</template>
          </p>
        </div>

        <p
            v-if="loading || dragging || identifying || checking"
            class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em]"
            :class="dragging ? 'text-acid' : 'text-fg-faint'"
        >
          <span class="size-1.5 animate-blink" :class="dragging ? 'bg-acid' : 'bg-fg-faint'"/>
          <template v-if="dragging">Отпустите файлы здесь</template>
          <template v-else-if="loading">Читаем папку</template>
          <template v-else-if="checking">Спрашиваем каталоги</template>
          <template v-else>Опознаём моды</template>
        </p>
      </div>

      <div
          v-if="updates.length"
          class="flex flex-wrap items-center justify-between gap-4 border border-acid/40 bg-ink-900 px-4 py-3"
      >
        <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-muted">
          Обновлений: {{ updates.length }}
        </p>

        <AppButton
            class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
            icon="i-lucide-arrow-up-circle"
            :loading="working"
            @click="applyUpdates(updates.map(update => update.path))"
        >
          Обновить все
        </AppButton>
      </div>

      <div
          v-if="picked.length"
          class="flex flex-wrap items-center justify-between gap-4 border border-line bg-ink-900 px-4 py-3"
      >
        <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-muted">
          Выбрано: {{ picked.length }}
        </p>

        <div class="flex items-center gap-4">
          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.18em]"
              icon="i-lucide-toggle-right"
              :disabled="working"
              @click="setPickedEnabled(true)"
          >
            Включить
          </AppButton>

          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.18em]"
              icon="i-lucide-toggle-left"
              :disabled="working"
              @click="setPickedEnabled(false)"
          >
            Выключить
          </AppButton>

          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.18em] hover:text-red-400"
              icon="i-lucide-trash-2"
              :disabled="working"
              @click="confirming = true"
          >
            Удалить
          </AppButton>

          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.18em]"
              icon="i-lucide-x"
              @click="selected = []"
          >
            Снять
          </AppButton>
        </div>
      </div>

      <div v-if="visible.length" class="border-t border-line">
        <div v-for="mod in visible" :key="mod.path" class="border-b border-line">
          <div
              class="group flex items-center gap-4 px-1 py-3 transition-colors duration-300 hover:bg-ink-700"
              :class="mod.enabled ? '' : 'opacity-55'"
          >
            <UCheckbox
                :model-value="selected.includes(modKey(mod))"
                @update:model-value="togglePicked(modKey(mod))"
            />

            <USwitch
                :model-value="mod.enabled"
                :disabled="busy.includes(mod.path) || working"
                @update:model-value="value => setEnabled(mod, value)"
            />

            <button
                type="button"
                class="flex min-w-0 flex-1 items-center gap-4 text-left"
                @click="toggle(modKey(mod))"
            >
              <InstanceModIcon
                  :icon-key="mod.details.iconKey"
                  :fallback-url="matchOf(mod.path)?.iconUrl"
                  :name="modName(mod, matchOf(mod.path))"
              />

              <span class="min-w-0 flex-1">
                <span class="flex items-center gap-2.5">
                  <span class="truncate text-[13px] font-medium text-fg" :title="mod.fileName">
                    {{ modName(mod, matchOf(mod.path)) }}
                  </span>

                  <span
                      v-if="updateOf(mod.path)"
                      class="shrink-0 border border-acid px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.2em] text-acid"
                  >
                    {{ updateOf(mod.path)?.to }}
                  </span>

                  <UIcon
                      v-if="mod.managed"
                      name="i-lucide-package"
                      class="size-3.5 shrink-0 text-fg-faint"
                      title="Мод из модпака"
                  />
                </span>

                <span class="mt-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">
                  <span class="truncate">{{ subtitle(mod) || mod.fileName }}</span>
                </span>
              </span>

              <span class="hidden shrink-0 items-center gap-2 sm:flex">
                <span
                    v-for="loader in loaders(mod)"
                    :key="loader"
                    class="border border-line px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-muted"
                >
                  {{ loader }}
                </span>
              </span>

              <UIcon
                  name="i-lucide-chevron-down"
                  class="size-4 shrink-0 text-fg-faint transition-transform duration-300"
                  :class="expanded === modKey(mod) ? 'rotate-180 text-acid' : ''"
              />
            </button>
          </div>

          <div v-if="expanded === modKey(mod)" class="space-y-3 px-1 pb-5 pl-[6.5rem] animate-rise">
            <p v-if="mod.details.description" class="text-[12px] leading-relaxed text-fg-muted">
              {{ mod.details.description }}
            </p>

            <dl class="grid gap-x-8 gap-y-2 sm:grid-cols-2">
              <div v-if="modAuthors(mod)" class="min-w-0">
                <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Авторы</dt>
                <dd class="truncate text-[12px] text-fg-muted">{{ modAuthors(mod) }}</dd>
              </div>

              <div v-if="mod.details.license" class="min-w-0">
                <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Лицензия</dt>
                <dd class="truncate text-[12px] text-fg-muted">{{ mod.details.license }}</dd>
              </div>

              <div class="min-w-0">
                <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Файл</dt>
                <dd class="truncate font-mono text-[11px] text-fg-muted" :title="mod.fileName">
                  {{ mod.fileName }}<template v-if="modSize(mod.size)"> · {{ modSize(mod.size) }}</template>
                </dd>
              </div>

              <div v-if="mod.details.homepage" class="min-w-0">
                <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Страница</dt>
                <dd>
                  <button
                      type="button"
                      class="truncate text-[12px] text-acid transition-opacity duration-300 hover:opacity-70"
                      @click="openHomepage(mod.details.homepage)"
                  >
                    {{ mod.details.homepage }}
                  </button>
                </dd>
              </div>
            </dl>

            <div v-if="matchOf(mod.path)" class="flex flex-wrap items-center gap-4">
              <button
                  v-if="matchOf(mod.path)?.pageUrl"
                  type="button"
                  class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-acid transition-opacity duration-300 hover:opacity-70"
                  @click="openHomepage(matchOf(mod.path)!.pageUrl)"
              >
                <UIcon name="i-lucide-external-link" class="size-3.5"/>
                {{ CATALOG_LABELS[matchOf(mod.path)!.provider] }}
              </button>

              <AppButton
                  v-if="updateOf(mod.path) && !mod.managed"
                  class="h-8 px-3 text-[10px] tracking-[0.18em]"
                  icon="i-lucide-arrow-up-circle"
                  :loading="working"
                  @click="applyUpdates([mod.path])"
              >
                Обновить до {{ updateOf(mod.path)?.to }}
              </AppButton>
            </div>

            <p v-if="mod.managed" class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
              <UIcon name="i-lucide-package" class="size-3.5"/>
              Мод из модпака - обновляется вместе с ним
            </p>
          </div>
        </div>
      </div>

      <div v-else class="flex items-center gap-3 border-t border-line py-10">
        <span class="size-1.5 bg-fg-faint animate-blink"/>
        <p class="font-mono text-[10px] uppercase tracking-[0.22em] text-fg-faint">
          <template v-if="loading || !loaded">Читаем папку модов</template>
          <template v-else-if="mods.length">Ничего не найдено</template>
          <template v-else-if="instance.type === 'vanilla'">Сборка без загрузчика модов</template>
          <template v-else>В папке mods пусто - перетащите сюда jar</template>
        </p>
      </div>
    </div>

    <InstanceModCatalog
        v-model:open="catalogOpen"
        :instance="instance"
        @installed="afterCatalogInstall"
    />

    <UModal
        :open="confirming"
        title="Удалить моды?"
        :ui="{ content: 'max-w-lg' }"
        @update:open="value => confirming = value"
    >
      <template #body>
        <div class="space-y-5">
          <p class="text-[12px] leading-relaxed text-fg-muted">
            Файлы будут удалены с диска без корзины. Отменить это нельзя.
          </p>

          <ul class="max-h-52 space-y-1.5 overflow-auto border border-line bg-ink-900 p-4">
            <li v-for="mod in picked" :key="mod.path" class="truncate font-mono text-[11px] text-fg-muted">
              {{ mod.fileName }}
            </li>
          </ul>

          <p
              v-if="pickedManaged.length"
              class="flex items-start gap-2 font-mono text-[10px] uppercase leading-relaxed tracking-[0.2em] text-amber-400"
          >
            <span class="mt-1 size-1.5 shrink-0 bg-amber-400 animate-blink"/>
            Из модпака: {{ pickedManaged.length }} - вернутся при следующем обновлении сборки
          </p>

          <div class="flex items-center justify-end gap-4 border-t border-line pt-5">
            <AppButton
                tone="quiet"
                class="text-[10px] tracking-[0.16em]"
                :disabled="working"
                @click="confirming = false"
            >
              Отмена
            </AppButton>

            <AppButton
                class="h-10 px-6 tracking-[0.18em] hover:border-red-400 hover:bg-red-400"
                icon="i-lucide-trash-2"
                :loading="working"
                @click="removePicked"
            >
              Удалить {{ picked.length }}
            </AppButton>
          </div>
        </div>
      </template>
    </UModal>
  </SettingsPanel>
</template>
