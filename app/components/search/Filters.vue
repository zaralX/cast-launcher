<script setup lang="ts">
import type {PackCapabilities, PackCategory, PackFilters, PackEnvironment} from "~/types/catalog";
import {categoryLabel} from "~/types/catalog";

const props = defineProps<{
  filters: PackFilters | null
  capabilities?: PackCapabilities | null
  loading?: boolean
}>()

const can = computed<PackCapabilities>(() => props.capabilities ?? {
  multipleGameVersions: true,
  environment: true,
  blockableFiles: false
})

const loaders = defineModel<string[]>("loaders", {required: true})
const gameVersions = defineModel<string[]>("gameVersions", {required: true})
const categories = defineModel<string[]>("categories", {required: true})
const environment = defineModel<PackEnvironment | null>("environment", {required: true})

const ENVIRONMENTS: { value: PackEnvironment | null, label: string }[] = [
  {value: null, label: "Любое"},
  {value: "client", label: "Клиент"},
  {value: "server", label: "Сервер"}
]

const groups = computed(() => {
  const byHeader = new Map<string, PackCategory[]>()

  for (const category of props.filters?.categories ?? []) {
    const header = category.header || "categories"
    byHeader.set(header, [...(byHeader.get(header) ?? []), category])
  }

  return [...byHeader.entries()].map(([header, items]) => ({header, items}))
})

const ANY_VERSION = "any"

const gameVersionItems = computed(() => [
  {label: "Любая", value: ANY_VERSION},
  ...(props.filters?.gameVersions ?? []).map(version => ({label: version, value: version}))
])

const singleGameVersion = computed({
  get: () => gameVersions.value[0] ?? ANY_VERSION,
  set: (value: string) => {
    gameVersions.value = value && value !== ANY_VERSION ? [value] : []
  }
})

const selectedCount = computed(() =>
    loaders.value.length + gameVersions.value.length + categories.value.length + (environment.value ? 1 : 0)
)

const toggled = (list: string[], value: string) =>
    list.includes(value) ? list.filter(item => item !== value) : [...list, value]

const toggleLoader = (value: string) => {
  loaders.value = toggled(loaders.value, value)
}

const toggleCategory = (value: string) => {
  categories.value = toggled(categories.value, value)
}

const selectEnvironment = (value: PackEnvironment | null) => {
  environment.value = value
}

const reset = () => {
  loaders.value = []
  gameVersions.value = []
  categories.value = []
  environment.value = null
}

const HEADER_LABELS: Record<string, string> = {
  categories: "Категории",
  resolutions: "Разрешение",
  "performance impact": "Влияние на производительность"
}
</script>

<template>
  <aside class="flex flex-col gap-7">
    <div class="flex items-center gap-3">
      <span class="font-mono text-[10px] uppercase tracking-[0.28em] text-acid">Фильтры</span>
      <span class="h-px flex-1 bg-line"/>
      <UButton
          v-if="selectedCount"
          color="neutral"
          variant="ghost"
          class="px-0 font-mono text-[9px] uppercase tracking-[0.16em] text-fg-faint hover:bg-transparent hover:text-acid"
          @click="reset"
      >
        Сбросить · {{ selectedCount }}
      </UButton>
    </div>

    <div v-if="loading" class="space-y-3">
      <span v-for="row in 4" :key="row" class="block h-7 w-full animate-pulse bg-ink-700"/>
    </div>

    <template v-else-if="filters">
      <section v-if="filters.loaders.length">
        <p class="mb-3 font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">Загрузчик</p>
        <div class="flex flex-wrap gap-2">
          <button
              v-for="loader in filters.loaders"
              :key="loader"
              type="button"
              :aria-pressed="loaders.includes(loader)"
              class="border px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] transition-colors duration-300"
              :class="loaders.includes(loader)
                ? 'border-acid text-acid'
                : 'border-line text-fg-faint hover:border-line-strong hover:text-fg-muted'"
              @click="toggleLoader(loader)"
          >
            {{ loader }}
          </button>
        </div>
      </section>

      <section v-if="filters.gameVersions.length">
        <p class="mb-3 font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">Версия игры</p>
        <USelectMenu
            v-if="can.multipleGameVersions"
            v-model="gameVersions"
            :items="filters.gameVersions"
            multiple
            placeholder="Любая"
            class="w-full"
        />
        <template v-else>
          <USelectMenu
              v-model="singleGameVersion"
              :items="gameVersionItems"
              value-key="value"
              class="w-full"
          />
          <p class="mt-2 text-[11px] leading-relaxed text-fg-faint">
            CurseForge ищет только по одной версии за раз.
          </p>
        </template>
      </section>

      <section v-if="can.environment">
        <p class="mb-3 font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">Окружение</p>
        <div class="grid grid-cols-3 border border-line">
          <button
              v-for="(option, i) in ENVIRONMENTS"
              :key="option.label"
              type="button"
              :aria-pressed="environment === option.value"
              class="py-2 font-mono text-[9px] uppercase tracking-[0.12em] transition-colors duration-300"
              :class="[
                i > 0 ? 'border-l border-line' : '',
                environment === option.value ? 'bg-ink-700 text-acid' : 'text-fg-faint hover:bg-ink-700/50 hover:text-fg-muted'
              ]"
              @click="selectEnvironment(option.value)"
          >
            {{ option.label }}
          </button>
        </div>
      </section>

      <section v-for="group in groups" :key="group.header">
        <p class="mb-3 font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">
          {{ HEADER_LABELS[group.header] ?? group.header }}
        </p>
        <div class="flex flex-wrap gap-2">
          <button
              v-for="category in group.items"
              :key="category.id"
              type="button"
              :aria-pressed="categories.includes(category.id)"
              class="border px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] transition-colors duration-300"
              :class="categories.includes(category.id)
                ? 'border-acid text-acid'
                : 'border-line text-fg-faint hover:border-line-strong hover:text-fg-muted'"
              @click="toggleCategory(category.id)"
          >
            {{ categoryLabel(category) }}
          </button>
        </div>
      </section>
    </template>

    <p v-else class="text-[12px] leading-relaxed text-fg-muted">
      Список фильтров не загрузился — поиск работает и без него.
    </p>
  </aside>
</template>
