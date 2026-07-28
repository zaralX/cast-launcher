<script setup lang="ts">
import type {ModrinthFilters, ModrinthHit, ModrinthSort, PackEnvironment} from "~/types/modrinth";
import {SORT_LABELS} from "~/types/modrinth";
import {call} from "~/types/backend";
import {LauncherError} from "~/types/error";

definePageMeta({
  layout: "main"
});

const PAGE_SIZE = 20
const DEBOUNCE = 350

type Source = "modrinth" | "curseforge"

const SOURCES: { value: Source, label: string, logo: string, ready: boolean }[] = [
  {value: "modrinth", label: "Modrinth", logo: "/modrinth.svg", ready: true},
  {value: "curseforge", label: "CurseForge", logo: "/curseforge.svg", ready: false}
]

const SORT_ITEMS = (Object.keys(SORT_LABELS) as ModrinthSort[]).map(value => ({
  label: SORT_LABELS[value],
  value
}))

const source = ref<Source>("modrinth")

const query = ref("")
const sort = ref<ModrinthSort>("relevance")
const loaders = ref<string[]>([])
const gameVersions = ref<string[]>([])
const categories = ref<string[]>([])
const environment = ref<PackEnvironment | null>(null)

const filters = ref<ModrinthFilters | null>(null)
const filtersLoading = ref(true)

const hits = ref<ModrinthHit[]>([])
const total = ref(0)
const searching = ref(false)
const loadingMore = ref(false)
const searchError = ref<LauncherError | null>(null)

const installTarget = ref<ModrinthHit | null>(null)
const installOpen = ref(false)

const hasMore = computed(() => hits.value.length < total.value)

/** Ответы могут прийти не в том порядке, в каком ушли запросы. */
let requestId = 0
let debounce: ReturnType<typeof setTimeout> | undefined

async function search(offset = 0) {
  const id = ++requestId

  if (offset === 0) searching.value = true
  else loadingMore.value = true

  searchError.value = null

  try {
    const page = await call("search_modrinth_packs", {
      query: {
        query: query.value,
        categories: categories.value,
        loaders: loaders.value,
        gameVersions: gameVersions.value,
        environment: environment.value,
        sort: sort.value,
        offset,
        limit: PAGE_SIZE
      }
    })

    if (id !== requestId) return

    hits.value = offset === 0 ? page.hits : [...hits.value, ...page.hits]
    total.value = page.totalHits
  } catch (e) {
    if (id !== requestId) return
    searchError.value = captureError(e, {code: "NETWORK", context: {action: "Поиск модпаков"}})
  } finally {
    if (id === requestId) {
      searching.value = false
      loadingMore.value = false
    }
  }
}

async function loadFilters() {
  filtersLoading.value = true
  filters.value = await safeRun(() => call("modrinth_filters"), {code: "NETWORK"}) ?? null
  filtersLoading.value = false
}

watch([query, sort, loaders, gameVersions, categories, environment], () => {
  clearTimeout(debounce)
  debounce = setTimeout(() => search(), DEBOUNCE)
}, {deep: true})

onMounted(() => {
  loadFilters()
  search()
})

onBeforeUnmount(() => clearTimeout(debounce))

const openInstall = (hit: ModrinthHit) => {
  installTarget.value = hit
  installOpen.value = true
}

const onInstalled = () => {
  installOpen.value = false
  navigateTo("/main")
}
</script>

<template>
  <div class="min-h-full w-full px-6 pb-10 pt-6 xl:px-10">
    <header class="animate-rise flex flex-wrap items-end justify-between gap-4 border-b border-line pb-5">
      <div>
        <p class="font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">Каталог</p>
        <h1 class="mt-2.5 font-unbounded text-lg font-bold leading-none tracking-[-0.055em] text-fg xl:text-xl">
          Поиск сборок<span class="text-acid">.</span>
        </h1>
      </div>

      <div role="radiogroup" aria-label="Источник сборок" class="flex border border-line">
        <button
            v-for="(item, i) in SOURCES"
            :key="item.value"
            type="button"
            role="radio"
            :aria-checked="source === item.value"
            :disabled="!item.ready"
            class="group relative flex items-center gap-2 px-4 py-2.5 transition-colors duration-300"
            :class="[
              i > 0 ? 'border-l border-line' : '',
              !item.ready ? 'cursor-not-allowed opacity-40' : '',
              source === item.value ? 'bg-ink-700 text-fg' : 'text-fg-faint hover:bg-ink-700/50 hover:text-fg-muted'
            ]"
            @click="item.ready && (source = item.value)"
        >
          <span
              class="absolute inset-x-0 top-0 h-px origin-center bg-acid transition-transform duration-500 ease-deck"
              :class="source === item.value ? 'scale-x-100' : 'scale-x-0'"
          />
          <NuxtImg :src="item.logo" class="size-3.5" alt=""/>
          <span class="font-mono text-[10px] uppercase tracking-[0.16em]">{{ item.label }}</span>
          <span v-if="!item.ready" class="font-mono text-[9px] tracking-[0.12em] text-fg-faint">скоро</span>
        </button>
      </div>
    </header>

    <section class="mt-6 grid gap-8 lg:grid-cols-[minmax(0,15rem)_minmax(0,1fr)]">
      <SearchFilters
          v-model:loaders="loaders"
          v-model:game-versions="gameVersions"
          v-model:categories="categories"
          v-model:environment="environment"
          :filters="filters"
          :loading="filtersLoading"
          class="animate-rise"
      />

      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-3">
          <UInput
              v-model="query"
              placeholder="Название модпака"
              icon="i-lucide-search"
              size="lg"
              class="min-w-0 flex-1"
              :loading="searching"
          />
          <USelect v-model="sort" :items="SORT_ITEMS" value-key="value" size="lg" class="w-52"/>
        </div>

        <div class="mt-4 flex items-center gap-4">
          <span class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
            {{ searching ? 'Идёт поиск' : `Найдено: ${total}` }}
          </span>
          <span class="h-px flex-1 bg-line"/>
        </div>

        <div v-if="searchError" class="mt-6 border border-red-400/30 bg-ink-900 p-5">
          <div class="flex items-start gap-3">
            <UIcon name="i-lucide-wifi-off" class="mt-0.5 size-4 shrink-0 text-red-400"/>
            <div class="min-w-0">
              <p class="text-[13px] font-medium text-fg">{{ searchError.title }}</p>
              <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
                {{ searchError.hint ?? searchError.message }}
              </p>
              <AppButton
                  tone="quiet"
                  class="mt-4 text-[10px] tracking-[0.18em]"
                  icon="i-lucide-rotate-cw"
                  @click="search()"
              >
                Повторить
              </AppButton>
            </div>
          </div>
        </div>

        <div v-else-if="searching && !hits.length" class="mt-4 grid gap-3 2xl:grid-cols-2">
          <span v-for="row in 6" :key="row" class="block h-[7.5rem] animate-pulse bg-ink-800"/>
        </div>

        <p v-else-if="!hits.length" class="mt-8 text-[13px] leading-relaxed text-fg-muted">
          Ничего не нашлось. Попробуйте смягчить фильтры или изменить запрос.
        </p>

        <div v-else class="mt-4 grid gap-3 2xl:grid-cols-2">
          <SearchPackCard
              v-for="(hit, i) in hits"
              :key="hit.projectId"
              :hit="hit"
              class="animate-rise"
              :style="{ animationDelay: `${Math.min(i, 12) * 30}ms` }"
              @install="openInstall"
          />
        </div>

        <div v-if="hasMore && !searchError" class="mt-6 flex justify-center">
          <AppButton
              class="h-10 px-8 tracking-[0.2em]"
              :loading="loadingMore"
              @click="search(hits.length)"
          >
            Показать ещё
          </AppButton>
        </div>
      </div>
    </section>

    <UModal v-model:open="installOpen" title="Установка сборки">
      <template #body>
        <SearchInstallModalBody
            v-if="installTarget"
            :key="installTarget.projectId"
            :hit="installTarget"
            @installed="onInstalled"
        />
      </template>
    </UModal>
  </div>
</template>
