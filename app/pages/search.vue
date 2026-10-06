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
  <div class="min-h-full w-full px-6 pt-6 pb-10 xl:px-10">
    <header class="animate-rise flex flex-wrap items-end justify-between gap-4 border-b border-line pb-5">
      <KitPageHeader
        :eyebrow="$t('search.eyebrow')"
        :title="$t('search.title')"
        size="xs"
      />

      <RadioGroup
        :model-value="source"
        :aria-label="$t('search.sources')"
        @update:model-value="value => selectSource(value as Source)"
      >
        <RadioGroupItem
          v-for="item in providers"
          :key="item.id"
          :value="item.id"
          :disabled="!item.ready"
          :title="item.reason ? uiText(item.reason) : undefined"
          class="flex-none px-4 py-2.5"
        >
          <img
            :src="PROVIDER_LOGOS[item.id]"
            class="size-3.5"
            alt=""
          >
          <span class="font-mono text-label uppercase tracking-caps">{{ item.label }}</span>
          <span
            v-if="!item.ready"
            class="font-mono text-micro tracking-[0.12em] text-fg-faint"
          >{{ $t('search.no_key') }}</span>
        </RadioGroupItem>

        <RadioGroupItem
          :value="CASTPACK"
          class="flex-none px-4 py-2.5"
        >
          <img
            src="/logo.svg"
            class="size-3.5"
            alt=""
          >
          <span class="font-mono text-label uppercase tracking-caps">CastPack</span>
        </RadioGroupItem>
      </RadioGroup>
    </header>

    <section
      v-if="source !== CASTPACK"
      class="mt-6 grid grid-cols-[minmax(0,15rem)_minmax(0,1fr)] gap-8"
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
          <KitSearchInput
            v-model="query"
            :placeholder="$t('search.placeholder')"
            :loading="searching"
            size="lg"
            class="min-w-0 flex-1"
          />
          <KitSelect
            v-model="sort"
            :items="SORT_ITEMS"
            size="lg"
            class="w-52"
          />
        </div>

        <div class="mt-4 flex items-center gap-4">
          <span class="font-mono text-label uppercase tracking-caps text-fg-faint">
            {{ searching ? $t('search.searching') : $t('search.found', { count: total }) }}
          </span>
          <Separator class="flex-1" />
        </div>

        <KitLoadError
          v-if="searchError"
          :error="searchError"
          class="mt-6"
          @retry="search()"
        />

        <div
          v-else-if="searching && !hits.length"
          class="mt-4 grid gap-3 2xl:grid-cols-2"
        >
          <Skeleton
            v-for="row in 6"
            :key="row"
            class="h-30 bg-ink-800"
          />
        </div>

        <p
          v-else-if="!hits.length"
          class="mt-8 text-title leading-relaxed text-fg-muted"
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
          <Button
            size="lg"
            class="px-8"
            :loading="loadingMore"
            @click="search(hits.length)"
          >
            {{ $t('search.show_more') }}
          </Button>
        </div>
      </div>
    </section>

    <section
      v-else
      class="mt-6"
    >
      <div class="flex flex-wrap items-center gap-3">
        <KitSearchInput
          v-model="catalogQuery"
          :placeholder="$t('search.catalog_placeholder')"
          size="lg"
          class="min-w-0 flex-1"
        />

        <Button
          variant="quiet"
          icon="i-lucide-rotate-cw"
          class="px-2"
          :loading="catalogLoading"
          @click="loadCatalog(true)"
        >
          {{ $t('search.refresh') }}
        </Button>
      </div>

      <Alert
        v-if="outdated"
        variant="warning"
        class="animate-rise mt-6"
      >
        {{ $t('search.outdated', { count: outdated }) }}
      </Alert>

      <KitLoading
        v-if="catalogLoading && !catalogLoaded"
        :label="$t('search.catalog_loading')"
        class="py-20"
      />

      <KitStatus
        v-else-if="!catalogPacks.length"
        class="py-20"
      >
        {{ catalogQuery ? $t('search.catalog_nothing_found') : $t('search.catalog_empty') }}
      </KitStatus>

      <div
        v-else
        class="mt-6 grid grid-cols-2 gap-3 2xl:grid-cols-3"
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
        class="mt-8 font-mono text-label uppercase tracking-caps text-fg-faint"
      >
        {{ $t('search.catalog_updated', { date: catalog.updatedAt }) }}
      </p>
    </section>

    <Dialog v-model:open="installOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ $t('search.install_title') }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <SearchInstallModalBody
            v-if="installTarget"
            :key="installTarget.projectId"
            :hit="installTarget"
            @installed="onInstalled"
          />
        </DialogBody>
      </DialogContent>
    </Dialog>
  </div>
</template>
