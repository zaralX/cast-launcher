<script setup lang="ts">
import type {Instance} from "~/types/instance"
import type {ModFile} from "~/types/mods"
import {MOD_LOADER_LABELS, matchesMod, modAuthors, modName, modSize} from "~/types/mods"
import {call} from "~/types/backend"

const props = defineProps<{ instance: Instance }>()

const ICON_BATCH = 10

type Filter = "all" | "enabled" | "disabled"

const modsStore = useModsStore()

const instanceId = computed(() => props.instance.id)

const mods = computed(() => modsStore.listOf(instanceId.value))
const loading = computed(() => modsStore.isLoading(instanceId.value))
const loaded = computed(() => modsStore.isLoaded(instanceId.value))

const query = ref("")
const filter = ref<Filter>("all")
const expanded = ref("")

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

function loaders(mod: ModFile) {
  return mod.details.loaders.map(loader => MOD_LOADER_LABELS[loader] ?? loader)
}

function subtitle(mod: ModFile) {
  return [mod.details.modId, mod.details.version].filter(part => part.trim()).join(" · ")
}

function toggle(path: string) {
  expanded.value = expanded.value === path ? "" : path
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

  if (result.ok) await loadIcons(result.value)
}

const openFolder = () => safeRun(
    () => call("open_instance_dir", {instanceId: instanceId.value, target: "mods"}),
    {context: {instanceId: instanceId.value, action: "Открытие папки модов"}}
)

const openHomepage = (url: string) => safeRun(
    () => call("open_url", {url}),
    {context: {instanceId: instanceId.value, action: "Открытие страницы мода"}}
)

watch(instanceId, () => {
  expanded.value = ""
  query.value = ""
  load()
})

onMounted(() => load())
</script>

<template>
  <SettingsPanel
      index="01"
      title="Моды"
      icon="i-lucide-blocks"
      description="Содержимое папки mods этой сборки: то, что лежит на диске, а не то, что заявлено в модпаке."
  >
    <div class="space-y-5">
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
        <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
          {{ visible.length }}<template v-if="visible.length !== mods.length"> из {{ mods.length }}</template>
          модов<template v-if="disabledCount"> · {{ disabledCount }} выключено</template>
        </p>

        <p v-if="loading" class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
          <span class="size-1.5 bg-acid animate-blink"/>
          Читаем папку
        </p>
      </div>

      <div v-if="visible.length" class="border-t border-line">
        <div v-for="mod in visible" :key="mod.path" class="border-b border-line">
          <button
              type="button"
              class="group flex w-full items-center gap-4 px-1 py-3 text-left transition-colors duration-300 hover:bg-ink-700"
              :class="mod.enabled ? '' : 'opacity-55'"
              @click="toggle(mod.path)"
          >
            <InstanceModIcon :icon-key="mod.details.iconKey" :name="modName(mod)"/>

            <span class="min-w-0 flex-1">
              <span class="flex items-center gap-2.5">
                <span class="truncate text-[13px] font-medium text-fg" :title="mod.fileName">
                  {{ modName(mod) }}
                </span>

                <span
                    v-if="!mod.enabled"
                    class="shrink-0 border border-line px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint"
                >
                  выкл
                </span>
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

          <div v-if="expanded === mod.path" class="space-y-3 px-1 pb-5 pl-14 animate-rise">
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
          </div>
        </div>
      </div>

      <div v-else class="flex items-center gap-3 border-t border-line py-10">
        <span class="size-1.5 bg-fg-faint animate-blink"/>
        <p class="font-mono text-[10px] uppercase tracking-[0.22em] text-fg-faint">
          <template v-if="loading || !loaded">Читаем папку модов</template>
          <template v-else-if="mods.length">Ничего не найдено</template>
          <template v-else-if="instance.type === 'vanilla'">Сборка без загрузчика модов</template>
          <template v-else>В папке mods пусто</template>
        </p>
      </div>
    </div>
  </SettingsPanel>
</template>
