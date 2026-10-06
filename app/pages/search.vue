<script setup lang="ts">
import type { PackFilters, PackHit, PackProviderInfo, PackSort, PackEnvironment } from '~/types/catalog'
import { PROVIDER_LOGOS, SORT_KEYS } from '~/types/catalog'
import type { PackProvider } from '~/types/instance'
import type { CatalogPack } from '~/types/castpack'
import type { LauncherError } from '~/utils/error'

definePageMeta({
  layout: 'main',
})

const { t } = useI18n()

type Source = PackProvider | 'castpack'

const CASTPACK = 'castpack' as const

const PAGE_SIZE = 20
const DEBOUNCE = 350

const route = useRoute()

const providers = ref<PackProviderInfo[]>([])

// source is the switcher choice, CastPack included; searchSource is the provider whose results are on screen.
const source = ref<Source>(route.query.source === CASTPACK ? CASTPACK : 'modrinth')
const searchSource = ref<PackProvider>('modrinth')

const provider = computed(() => providers.value.find(item => item.id === searchSource.value) ?? null)

const SORT_ITEMS = computed(() =>
  (provider.value?.sorts ?? (Object.keys(SORT_KEYS) as PackSort[])).map(value => ({
    label: t(SORT_KEYS[value]!),
    value,
  })),
)

const query = ref('')
const sort = ref<PackSort>('relevance')
const loaders = ref<string[]>([])
const gameVersions = ref<string[]>([])
const categories = ref<string[]>([])
const environment = ref<PackEnvironment | null>(null)

const filters = ref<PackFilters | null>(null)
const filtersLoading = ref(true)

const hits = ref<PackHit[]>([])
const total = ref(0)
const searching = ref(false)
const loadingMore = ref(false)
const searchError = ref<LauncherError | null>(null)

const installTarget = ref<PackHit | null>(null)
const installOpen = ref(false)

const hasMore = computed(() => hits.value.length < total.value)

let requestId = 0
let debounce: ReturnType<typeof setTimeout> | undefined

async function search(offset = 0) {
  const id = ++requestId

  if (offset === 0) searching.value = true
  else loadingMore.value = true

  searchError.value = null

  try {
    const page = await call('search_packs', {
      query: {
        provider: searchSource.value,
        query: query.value,
        categories: categories.value,
        loaders: loaders.value,
        gameVersions: gameVersions.value,
        environment: environment.value,
        sort: sort.value,
        offset,
        limit: PAGE_SIZE,
      },
    })

    if (id !== requestId) return

    hits.value = offset === 0 ? page.hits : [...hits.value, ...page.hits]
    total.value = page.totalHits
  }
  catch (e) {
    if (id !== requestId) return
    searchError.value = captureError(e, { code: 'NETWORK', context: { action: t('search.search_action') } })
  }
  finally {
    if (id === requestId) {
      searching.value = false
      loadingMore.value = false
    }
  }
}

async function loadFilters() {
  filtersLoading.value = true
  filters.value = await safeRun(() => call('pack_filters', { provider: searchSource.value }), { code: 'NETWORK' }) ?? null
  filtersLoading.value = false
}

async function loadProviders() {
  const list = await safeRun(() => call('pack_providers'), { code: 'NETWORK' })
  providers.value = list ?? []

  if (providers.value.some(item => item.id === searchSource.value && item.ready)) return

  const fallback = providers.value.find(item => item.ready)
  if (!fallback) return

  searchSource.value = fallback.id
  if (source.value !== CASTPACK) source.value = fallback.id
}

async function selectSource(next: Source) {
  if (next === source.value) return

  source.value = next

  navigateTo({ query: next === CASTPACK ? { source: next } : {} }, { replace: true })

  if (next === CASTPACK) {
    loadCatalog()
    return
  }

  if (next === searchSource.value) return

  searchSource.value = next

  categories.value = []
  gameVersions.value = []
  loaders.value = []
  environment.value = null

  if (!provider.value?.sorts.includes(sort.value)) {
    sort.value = 'relevance'
  }

  hits.value = []
  total.value = 0

  clearTimeout(debounce)

  await loadFilters()
  await search()
}

watch([query, sort, loaders, gameVersions, categories, environment], () => {
  clearTimeout(debounce)
  debounce = setTimeout(() => search(), DEBOUNCE)
}, { deep: true })

onMounted(async () => {
  if (source.value === CASTPACK) loadCatalog()

  await loadProviders()
  loadFilters()
  search()
})

onBeforeUnmount(() => clearTimeout(debounce))

const openInstall = (hit: PackHit) => {
  installTarget.value = hit
  installOpen.value = true
}

const onInstalled = () => {
  installOpen.value = false
  navigateTo('/main')
}

const castpackStore = useCastPackStore()
const instanceStore = useInstanceStore()
const toast = useAppToast()

const { catalog, loading: catalogLoading, loaded: catalogLoaded } = storeToRefs(castpackStore)

const catalogQuery = ref('')

const catalogPacks = computed<CatalogPack[]>(() => {
  const needle = catalogQuery.value.trim().toLowerCase()
  if (!needle) return castpackStore.packs

  return castpackStore.packs.filter(pack =>
    [pack.name, pack.summary, pack.description, pack.id, ...pack.tags]
      .join(' ')
      .toLowerCase()
      .includes(needle),
  )
})

const outdated = computed(() =>
  castpackStore.packs.filter(pack => castpackStore.stateOf(pack) === 'outdated').length,
)

const packInstallOf = (packId: string) => {
  const instance = castpackStore.instanceOf(packId)
  return instance ? instanceStore.getInstall(instance.id) : undefined
}

async function loadCatalog(force = false) {
  await safeRun(() => castpackStore.loadCatalog(force), {
    code: 'NETWORK',
    context: { action: t('search.catalog_action') },
  })
}

async function installPack(packId: string) {
  const pack = castpackStore.packs.find(item => item.id === packId)

  const started = await attempt(() => castpackStore.installPack(packId), {
    context: { action: t('search.install_action'), packId },
  })

  if (!started.ok) return

  toast.add({
    title: t('search.installing', { name: pack?.name ?? packId }),
    description: t('search.installing_hint'),
    color: 'success',
    icon: 'i-lucide-arrow-down-to-line',
  })
}

const playPack = (instanceId: string) => safeRun(
  () => instanceStore.playInstance(instanceId),
  { context: { instanceId, action: t('instance.context.play') } },
)
</script>

<template>
  <div class="min-h-full w-full px-6 pb-10 pt-6 xl:px-10">
    <header class="animate-rise flex flex-wrap items-end justify-between gap-4 border-b border-line pb-5">
      <div>
        <p class="font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">
          {{ $t('search.eyebrow') }}
        </p>
        <h1 class="mt-2.5 font-unbounded text-lg font-bold leading-none tracking-[-0.055em] text-fg xl:text-xl">
          {{ $t('search.title') }}<span class="text-acid">.</span>
        </h1>
      </div>

      <div
        role="radiogroup"
        :aria-label="$t('search.sources')"
        class="flex border border-line"
      >
        <button
          v-for="item in providers"
          :key="item.id"
          type="button"
          role="radio"
          :aria-checked="source === item.id"
          :disabled="!item.ready"
          :title="item.reason ? uiText(item.reason) : undefined"
          class="group relative flex items-center gap-2 border-r border-line px-4 py-2.5 transition-colors duration-300"
          :class="[
            !item.ready ? 'cursor-not-allowed opacity-40' : '',
            source === item.id ? 'bg-ink-700 text-fg' : 'text-fg-faint hover:bg-ink-700/50 hover:text-fg-muted',
          ]"
          @click="item.ready && selectSource(item.id)"
        >
          <span
            class="absolute inset-x-0 top-0 h-px origin-center bg-acid transition-transform duration-500 ease-deck"
            :class="source === item.id ? 'scale-x-100' : 'scale-x-0'"
          />
          <img
            :src="PROVIDER_LOGOS[item.id]"
            class="size-3.5"
            alt=""
          >
          <span class="font-mono text-[10px] uppercase tracking-[0.16em]">{{ item.label }}</span>
          <span
            v-if="!item.ready"
            class="font-mono text-[9px] tracking-[0.12em] text-fg-faint"
          >{{ $t('search.no_key') }}</span>
        </button>

        <button
          type="button"
          role="radio"
          :aria-checked="source === CASTPACK"
          class="group relative flex items-center gap-2 px-4 py-2.5 transition-colors duration-300"
          :class="source === CASTPACK
            ? 'bg-ink-700 text-fg'
            : 'text-fg-faint hover:bg-ink-700/50 hover:text-fg-muted'"
          @click="selectSource(CASTPACK)"
        >
          <span
            class="absolute inset-x-0 top-0 h-px origin-center bg-acid transition-transform duration-500 ease-deck"
            :class="source === CASTPACK ? 'scale-x-100' : 'scale-x-0'"
          />
          <img
            src="/logo.svg"
            class="size-3.5"
            alt=""
          >
          <span class="font-mono text-[10px] uppercase tracking-[0.16em]">CastPack</span>
        </button>
      </div>
    </header>

    <section
      v-if="source !== CASTPACK"
      class="mt-6 grid gap-8 lg:grid-cols-[minmax(0,15rem)_minmax(0,1fr)]"
    >
      <SearchFilters
        v-model:loaders="loaders"
        v-model:game-versions="gameVersions"
        v-model:categories="categories"
        v-model:environment="environment"
        :filters="filters"
        :loading="filtersLoading"
        :capabilities="provider?.capabilities ?? null"
        class="animate-rise"
      />

      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-3">
          <UInput
            v-model="query"
            :placeholder="$t('search.placeholder')"
            icon="i-lucide-search"
            size="lg"
            class="min-w-0 flex-1"
            :loading="searching"
          />
          <USelect
            v-model="sort"
            :items="SORT_ITEMS"
            value-key="value"
            size="lg"
            class="w-52"
          />
        </div>

        <div class="mt-4 flex items-center gap-4">
          <span class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
            {{ searching ? $t('search.searching') : $t('search.found', { count: total }) }}
          </span>
          <span class="h-px flex-1 bg-line" />
        </div>

        <div
          v-if="searchError"
          class="mt-6 border border-red-400/30 bg-ink-900 p-5"
        >
          <div class="flex items-start gap-3">
            <UIcon
              name="i-lucide-wifi-off"
              class="mt-0.5 size-4 shrink-0 text-red-400"
            />
            <div class="min-w-0">
              <p class="text-[13px] font-medium text-fg">
                {{ searchError.title }}
              </p>
              <p
                v-if="searchError.reason ?? searchError.hint"
                class="mt-2 text-[12px] leading-relaxed text-fg-muted"
              >
                {{ searchError.reason ?? searchError.hint }}
              </p>
              <AppButton
                tone="quiet"
                class="mt-4 text-[10px] tracking-[0.18em]"
                icon="i-lucide-rotate-cw"
                @click="search()"
              >
                {{ $t('search.retry') }}
              </AppButton>
            </div>
          </div>
        </div>

        <div
          v-else-if="searching && !hits.length"
          class="mt-4 grid gap-3 2xl:grid-cols-2"
        >
          <span
            v-for="row in 6"
            :key="row"
            class="block h-[7.5rem] animate-pulse bg-ink-800"
          />
        </div>

        <p
          v-else-if="!hits.length"
          class="mt-8 text-[13px] leading-relaxed text-fg-muted"
        >
          {{ $t('search.nothing_found') }}
        </p>

        <div
          v-else
          class="mt-4 grid gap-3 2xl:grid-cols-2"
        >
          <SearchPackCard
            v-for="(hit, i) in hits"
            :key="hit.projectId"
            :hit="hit"
            class="animate-rise"
            :style="{ animationDelay: `${Math.min(i, 12) * 30}ms` }"
            @install="openInstall"
          />
        </div>

        <div
          v-if="hasMore && !searchError"
          class="mt-6 flex justify-center"
        >
          <AppButton
            class="h-10 px-8 tracking-[0.2em]"
            :loading="loadingMore"
            @click="search(hits.length)"
          >
            {{ $t('search.show_more') }}
          </AppButton>
        </div>
      </div>
    </section>

    <section
      v-else
      class="mt-6"
    >
      <div class="flex flex-wrap items-center gap-3">
        <UInput
          v-model="catalogQuery"
          :placeholder="$t('search.catalog_placeholder')"
          icon="i-lucide-search"
          size="lg"
          class="min-w-0 flex-1"
        />

        <AppButton
          tone="quiet"
          class="h-10 shrink-0 px-5 text-[10px] tracking-[0.18em]"
          icon="i-lucide-rotate-cw"
          :loading="catalogLoading"
          @click="loadCatalog(true)"
        >
          {{ $t('search.refresh') }}
        </AppButton>
      </div>

      <p
        v-if="outdated"
        class="animate-rise mt-6 border border-amber-400/30 bg-amber-400/[0.04] px-4 py-3 text-[12px] leading-relaxed text-fg-muted"
      >
        {{ $t('search.outdated', { count: outdated }) }}
      </p>

      <div
        v-if="catalogLoading && !catalogLoaded"
        class="flex flex-col items-center gap-4 py-20"
      >
        <span class="relative block h-px w-40 overflow-hidden bg-line">
          <span class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep" />
        </span>
        <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
          {{ $t('search.catalog_loading') }}
        </p>
      </div>

      <div
        v-else-if="!catalogPacks.length"
        class="flex items-center gap-3 py-20"
      >
        <span class="size-1.5 bg-fg-faint animate-blink" />
        <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
          {{ catalogQuery ? $t('search.catalog_nothing_found') : $t('search.catalog_empty') }}
        </p>
      </div>

      <div
        v-else
        class="mt-6 grid gap-3 lg:grid-cols-2 2xl:grid-cols-3"
      >
        <CastpackCard
          v-for="(pack, i) in catalogPacks"
          :key="pack.id"
          :pack="pack"
          :state="castpackStore.stateOf(pack)"
          :icon="castpackStore.instanceOf(pack.id)?.icon"
          :instance-id="castpackStore.instanceOf(pack.id)?.id"
          :progress="packInstallOf(pack.id)?.progress"
          :phase="packInstallOf(pack.id)?.phase"
          class="animate-rise"
          :style="{ animationDelay: `${Math.min(i, 12) * 45}ms` }"
          @install="installPack"
          @play="playPack"
        />
      </div>

      <p
        v-if="catalog?.updatedAt"
        class="mt-8 font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint"
      >
        {{ $t('search.catalog_updated', { date: catalog.updatedAt }) }}
      </p>
    </section>

    <UModal
      v-model:open="installOpen"
      :title="$t('search.install_title')"
    >
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
