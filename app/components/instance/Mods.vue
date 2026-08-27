<script setup lang="ts">
import {getCurrentWebview} from "@tauri-apps/api/webview"
import type {UnlistenFn} from "@tauri-apps/api/event"
import type {Instance} from "~/types/instance"
import type {InstalledMods, ModFile} from "~/types/mods"
import {MOD_LOADER_LABELS, isModFile, matchesMod, modAuthors, modName, modSize} from "~/types/mods"
import {call} from "~/types/backend"

const props = defineProps<{ instance: Instance }>()

const ICON_BATCH = 10

type Filter = "all" | "enabled" | "disabled"

const modsStore = useModsStore()
const toast = useToast()

const instanceId = computed(() => props.instance.id)

const mods = computed(() => modsStore.listOf(instanceId.value))
const loading = computed(() => modsStore.isLoading(instanceId.value))
const loaded = computed(() => modsStore.isLoaded(instanceId.value))

const query = ref("")
const filter = ref<Filter>("all")
const expanded = ref("")
const selected = ref<string[]>([])
const busy = ref<string[]>([])
const working = ref(false)
const confirming = ref(false)
const dragging = ref(false)

const FILTERS = [
  {label: "Все", value: "all"},
  {label: "Включённые", value: "enabled"},
  {label: "Выключенные", value: "disabled"}
]

const disabledCount = computed(() => mods.value.filter(mod => !mod.enabled).length)

const visible = computed(() => mods.value.filter(mod => {
  if (filter.value === "enabled" && !mod.enabled) return false
  if (filter.value === "disabled" && mod.enabled) return false

  return matchesMod(mod, query.value)
}))

const picked = computed(() => mods.value.filter(mod => selected.value.includes(mod.path)))
const pickedManaged = computed(() => picked.value.filter(mod => mod.managed))

const allVisiblePicked = computed(() =>
    visible.value.length > 0 && visible.value.every(mod => selected.value.includes(mod.path))
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
  const paths = visible.value.map(mod => mod.path)

  selected.value = allVisiblePicked.value
      ? selected.value.filter(path => !paths.includes(path))
      : [...new Set([...selected.value, ...paths])]
}

function keepAlive() {
  const paths = mods.value.map(mod => mod.path)

  selected.value = selected.value.filter(path => paths.includes(path))
  if (!paths.includes(expanded.value)) expanded.value = ""
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
    installed.skipped.length ? `пропущено ${installed.skipped.length}` : ""
  ].filter(Boolean)

  toast.add({
    title: parts.length ? parts.join(", ") : "Ничего не добавлено",
    description: installed.skipped.length ? installed.skipped.join(", ") : undefined,
    color: installed.added.length || installed.replaced.length ? "success" : "warning",
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
              icon="i-lucide-package-plus"
              :loading="working"
              @click="pickFiles"
          >
            Добавить
          </AppButton>

          <AppButton
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              icon="i-lucide-refresh-cw"
              :loading="loading"
              @click="load(true)"
          >
            Обновить
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
            v-if="loading || dragging"
            class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em]"
            :class="dragging ? 'text-acid' : 'text-fg-faint'"
        >
          <span class="size-1.5 animate-blink" :class="dragging ? 'bg-acid' : 'bg-fg-faint'"/>
          {{ dragging ? 'Отпустите файлы здесь' : 'Читаем папку' }}
        </p>
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
                :model-value="selected.includes(mod.path)"
                @update:model-value="togglePicked(mod.path)"
            />

            <USwitch
                :model-value="mod.enabled"
                :disabled="busy.includes(mod.path) || working"
                @update:model-value="value => setEnabled(mod, value)"
            />

            <button
                type="button"
                class="flex min-w-0 flex-1 items-center gap-4 text-left"
                @click="toggle(mod.path)"
            >
              <InstanceModIcon :icon-key="mod.details.iconKey" :name="modName(mod)"/>

              <span class="min-w-0 flex-1">
                <span class="flex items-center gap-2.5">
                  <span class="truncate text-[13px] font-medium text-fg" :title="mod.fileName">
                    {{ modName(mod) }}
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
                  :class="expanded === mod.path ? 'rotate-180 text-acid' : ''"
              />
            </button>
          </div>

          <div v-if="expanded === mod.path" class="space-y-3 px-1 pb-5 pl-[6.5rem] animate-rise">
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

            <p v-if="mod.managed" class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
              <UIcon name="i-lucide-package" class="size-3.5"/>
              Мод из модпака
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
