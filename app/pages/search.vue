<script setup lang="ts">
import type { PackFilters, PackHit, PackProviderInfo, PackSort, PackEnvironment } from '~/types/catalog'
import { PROVIDER_LOGOS, SORT_KEYS } from '~/types/catalog'
import type { PackProvider } from '~/types/instance'
import type { CatalogPack } from '~/types/castpack'
import type { LauncherError } from '~/utils/error'

definePageMeta({
  layout: 'main',
})

const { t, locale } = useI18n()

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

const catalogUpdated = computed(() => {
  const value = catalog.value?.updatedAt
  return value ? new Date(value).toLocaleString(locale.value) : ''
})

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
  <div class="min-h-full w-full px-8 pt-6 pb-10">
    <div class="flex items-center gap-2">
      <RadioGroup
        :model-value="source"
        :aria-label="$t('search.sources')"
        class="mr-2"
        @update:model-value="value => selectSource(value as Source)"
      >
        <RadioGroupItem
          v-for="item in providers"
          :key="item.id"
          :value="item.id"
          :disabled="!item.ready"
          :title="item.reason ? uiText(item.reason) : undefined"
          class="h-8.5 flex-none gap-1.5"
        >
          <img
            :src="PROVIDER_LOGOS[item.id]"
            class="size-3.5"
            alt=""
          >
          {{ item.label }}
          <span
            v-if="!item.ready"
            class="font-normal text-fg-faint"
          >{{ $t('search.no_key') }}</span>
        </RadioGroupItem>

        <RadioGroupItem
          :value="CASTPACK"
          class="h-8.5 flex-none gap-1.5"
        >
          <img
            src="/logo.svg"
            class="size-3.5"
            alt=""
          >
          CastPack
        </RadioGroupItem>
      </RadioGroup>

      <template v-if="source !== CASTPACK">
        <KitSearchInput
          v-model="query"
          :placeholder="$t('search.placeholder')"
          :aria-label="$t('search.placeholder')"
          :loading="searching"
          size="lg"
          class="min-w-0 flex-1"
        />
        <KitSelect
          v-model="sort"
          :items="SORT_ITEMS"
          size="lg"
          class="w-49"
        />
      </template>

      <template v-else>
        <KitSearchInput
          v-model="catalogQuery"
          :placeholder="$t('search.catalog_placeholder')"
          :aria-label="$t('search.catalog_placeholder')"
          size="lg"
          class="min-w-0 flex-1"
        />
        <Button
          variant="ghost"
          size="lg"
          icon="i-lucide-rotate-cw"
          class="px-3.5 text-body"
          :loading="catalogLoading"
          @click="loadCatalog(true)"
        >
          {{ $t('search.refresh') }}
        </Button>
      </template>
    </div>

    <section
      v-if="source !== CASTPACK"
      class="mt-5 grid grid-cols-[13.75rem_minmax(0,1fr)] gap-8"
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
        <p class="flex h-7 items-center text-caption text-fg-faint">
          {{ searching ? $t('search.searching') : $t('search.found', { count: total.toLocaleString(locale) }) }}
        </p>

        <KitLoadError
          v-if="searchError"
          :error="searchError"
          class="mt-3"
          @retry="search()"
        />

        <div
          v-else-if="searching && !hits.length"
          class="mt-3 space-y-3.5"
        >
          <div
            v-for="row in 6"
            :key="row"
            class="flex gap-4 py-1"
          >
            <Skeleton class="size-14 shrink-0 bg-panel" />
            <div class="flex-1 space-y-2 pt-0.5">
              <Skeleton class="h-3.5 w-1/2 bg-panel" />
              <Skeleton class="h-3 w-5/6 bg-panel" />
              <Skeleton class="h-3 w-1/3 bg-panel" />
            </div>
          </div>
        </div>

        <p
          v-else-if="!hits.length"
          class="mt-3 max-w-[46ch] text-title text-fg-muted"
        >
          {{ $t('search.nothing_found') }}
        </p>

        <div
          v-else
          class="-mx-3.5 mt-1"
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
          class="mt-3.5 flex justify-center"
        >
          <Button
            size="lg"
            class="px-7"
            :loading="loadingMore"
            @click="search(hits.length)"
          >
            {{ $t('search.show_more') }}
          </Button>
        </div>
      </div>
    </section>

    <section v-else>
      <Alert
        v-if="outdated"
        variant="warning"
        icon="i-lucide-triangle-alert"
        class="animate-rise mt-4"
      >
        {{ $t('search.outdated', outdated) }}
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
        class="mt-4 grid grid-cols-2 gap-3 2xl:grid-cols-3"
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
        v-if="catalogUpdated"
        class="mt-5 text-caption text-fg-faint"
      >
        {{ $t('search.catalog_updated', { date: catalogUpdated }) }}
      </p>
    </section>

    <Dialog v-model:open="installOpen">
      <DialogContent class="max-w-120">
        <SearchInstallModalBody
          v-if="installTarget"
          :key="installTarget.projectId"
          :hit="installTarget"
          @installed="onInstalled"
        />
      </DialogContent>
    </Dialog>
  </div>
</template>
