<script setup lang="ts">
import type { Instance } from '~/types/instance'
import type { PackHit, PackProviderInfo } from '~/types/catalog'
import type { CatalogVersion, InstallPlan } from '~/types/mods'
import { RELEASE_KEYS } from '~/types/mods'
import { PROVIDER_LOGOS } from '~/types/catalog'
import { INSTANCE_TYPE_LABELS } from '~/types/instance'

const props = defineProps<{ instance: Instance, open: boolean }>()
const emit = defineEmits<{ 'update:open': [boolean], 'installed': [] }>()

type Provider = 'modrinth' | 'curseforge'

const PAGE = 20
const DEBOUNCE = 350

const PROVIDERS: { label: string, value: Provider }[] = [
  { label: 'Modrinth', value: 'modrinth' },
  { label: 'CurseForge', value: 'curseforge' },
]

const RELEASE_TONE: Record<string, 'success' | 'warning' | 'danger'> = {
  release: 'success',
  beta: 'warning',
  alpha: 'danger',
}

const { t, locale } = useI18n()

const SORTS = computed(() => [
  { label: t('mod_catalog.sort.relevance'), value: 'relevance' },
  { label: t('mod_catalog.sort.downloads'), value: 'downloads' },
  { label: t('mod_catalog.sort.updated'), value: 'updated' },
])

const modsStore = useModsStore()
const toast = useAppToast()

const instanceId = computed(() => props.instance.id)

const providers = ref<PackProviderInfo[]>([])
const provider = ref<Provider>('modrinth')
const query = ref('')
const sort = ref('relevance')

const hits = ref<PackHit[]>([])
const total = ref(0)
const searching = ref(false)

const chosen = ref<PackHit | null>(null)
const versions = ref<CatalogVersion[]>([])
const loadingVersions = ref(false)

const plan = ref<InstallPlan | null>(null)
const planning = ref(false)
const optional = ref<string[]>([])
const installing = ref(false)

const providerInfo = computed(() => providers.value.find(item => item.id === provider.value))
const providerReady = computed(() => providerInfo.value?.ready ?? true)

const installed = computed(() => {
  const matches = modsStore.catalog[instanceId.value] ?? {}

  return new Set(Object.values(matches).map(matched => matched.projectId))
})

const stage = computed(() => {
  if (plan.value) return 'plan'
  if (chosen.value) return 'versions'
  return 'search'
})

const title = computed(() => {
  if (stage.value === 'plan') return t('mod_catalog.title.plan')
  if (stage.value === 'versions') return chosen.value?.title ?? t('mod_catalog.title.versions')
  return t('mod_catalog.title.search')
})

const loader = computed(() => `${INSTANCE_TYPE_LABELS[props.instance.type] ?? props.instance.type} ${props.instance.minecraftVersion}`)

const subtitle = computed(() => {
  if (stage.value === 'versions') {
    return [chosen.value?.author, t('mod_catalog.versions_for', { loader: loader.value })].filter(Boolean).join(' · ')
  }
  return t('mod_catalog.target', { name: props.instance.name, loader: loader.value })
})

const installCount = computed(() => plan.value ? 1 + plan.value.required.length + optional.value.length : 0)

let timer: ReturnType<typeof setTimeout> | null = null

function back() {
  if (plan.value) {
    plan.value = null
    optional.value = []
    return
  }

  chosen.value = null
  versions.value = []
}

async function search(offset = 0) {
  if (!providerReady.value) return

  searching.value = true

  const result = await attempt(
    () => modsStore.searchMods(instanceId.value, {
      provider: provider.value,
      query: query.value,
      sort: sort.value,
      offset,
      limit: PAGE,
    }),
    { context: { instanceId: instanceId.value, action: t('mod_catalog.search_action') } },
  )

  searching.value = false

  if (!result.ok) return

  hits.value = offset ? [...hits.value, ...result.value.hits] : result.value.hits
  total.value = result.value.totalHits
}

function searchLater() {
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => search(), DEBOUNCE)
}

async function choose(hit: PackHit) {
  chosen.value = hit
  versions.value = []
  loadingVersions.value = true

  const result = await attempt(
    () => modsStore.modVersions(instanceId.value, provider.value, hit.projectId),
    { context: { instanceId: instanceId.value, action: t('mod_catalog.versions_action') } },
  )

  loadingVersions.value = false

  if (result.ok) versions.value = result.value
}

async function pickVersion(version: CatalogVersion) {
  if (!chosen.value) return

  planning.value = true

  const result = await attempt(
    () => modsStore.planInstall(instanceId.value, provider.value, chosen.value!.projectId, version.versionId),
    { context: { instanceId: instanceId.value, action: t('mod_catalog.plan_action') } },
  )

  planning.value = false

  if (!result.ok) return

  plan.value = result.value
  optional.value = []
}

function toggleOptional(projectId: string) {
  optional.value = optional.value.includes(projectId)
    ? optional.value.filter(id => id !== projectId)
    : [...optional.value, projectId]
}

async function install() {
  if (!plan.value || installing.value) return

  installing.value = true

  const result = await attempt(
    () => modsStore.installMod(instanceId.value, plan.value!.id, optional.value),
    { context: { instanceId: instanceId.value, action: t('mod_catalog.install_action') } },
  )

  installing.value = false

  if (!result.ok) return

  const { installed: added, failed, blocked } = result.value

  toast.add({
    title: added.length ? t('mod_catalog.installed_count', { count: added.length }) : t('mod_catalog.nothing_installed'),
    description: [
      failed.length ? t('mod_catalog.failed', { list: failed.join(', ') }) : '',
      blocked.length ? t('mod_catalog.manual', { list: blocked.join(', ') }) : '',
    ].filter(Boolean).join('; ') || undefined,
    color: failed.length || blocked.length ? 'warning' : 'success',
    icon: 'i-lucide-package-plus',
  })

  emit('installed')

  plan.value = null
  optional.value = []
  chosen.value = null
}

const openPage = (url: string) => safeRun(
  () => call('open_url', { url }),
  { context: { instanceId: instanceId.value, action: t('mod_catalog.open_page_action') } },
)

function downloads(value: number) {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${Math.round(value / 1_000)}K`

  return String(value)
}

watch(() => props.open, async (open) => {
  if (!open) return

  if (!providers.value.length) {
    const result = await attempt(() => call('pack_providers'), { context: { action: t('mod_catalog.providers_action') } })
    if (result.ok) providers.value = result.value
  }

  if (!hits.value.length) await search()
})

watch([provider, sort], () => {
  hits.value = []
  back()
  search()
})

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})
</script>

<template>
  <Dialog
    :open="open"
    @update:open="value => emit('update:open', value)"
  >
    <DialogContent class="max-w-190">
      <DialogHeader class="flex-row items-start gap-3">
        <Button
          v-if="stage !== 'search'"
          variant="ghost"
          size="icon-sm"
          icon="i-lucide-arrow-left"
          :aria-label="$t('mod_catalog.back')"
          :disabled="installing"
          class="-ml-2"
          @click="back"
        />
        <KitThumb
          v-if="stage === 'versions' && chosen"
          :src="chosen.iconUrl"
          :alt="chosen.title"
          size="sm"
        />
        <div class="min-w-0 flex-1">
          <DialogTitle class="truncate leading-snug">
            {{ title }}
          </DialogTitle>
          <DialogDescription class="mt-0.5 truncate text-caption">
            {{ subtitle }}
          </DialogDescription>
        </div>
      </DialogHeader>

      <DialogBody class="space-y-4">
        <Alert
          v-if="instance.type === 'vanilla'"
          variant="warning"
          icon="i-lucide-triangle-alert"
        >
          {{ $t('mod_catalog.no_loader') }}
        </Alert>

        <template v-if="stage === 'search'">
          <div class="flex items-center gap-2">
            <KitSearchInput
              v-model="query"
              :loading="searching"
              :placeholder="$t('mod_catalog.search_placeholder')"
              :aria-label="$t('mod_catalog.search_placeholder')"
              size="lg"
              class="min-w-0 flex-1"
              @update:model-value="searchLater"
              @keydown.enter="search()"
            />
            <RadioGroup
              v-model="provider"
              :aria-label="$t('mod_catalog.provider')"
            >
              <RadioGroupItem
                v-for="item in PROVIDERS"
                :key="item.value"
                :value="item.value"
                class="h-8.5 flex-none gap-1.5"
              >
                <img
                  :src="PROVIDER_LOGOS[item.value]"
                  class="size-3.5"
                  alt=""
                >
                {{ item.label }}
              </RadioGroupItem>
            </RadioGroup>
            <KitSelect
              v-model="sort"
              :items="SORTS"
              :aria-label="$t('mod_catalog.sort')"
              size="lg"
              class="w-44"
            />
          </div>

          <KitStatus
            v-if="!providerReady"
            tone="warning"
          >
            {{ providerInfo?.reason ? uiText(providerInfo.reason) : $t('mod_catalog.unavailable') }}
          </KitStatus>

          <template v-else>
            <div
              v-if="searching && !hits.length"
              class="space-y-3 py-1"
            >
              <div
                v-for="row in 5"
                :key="row"
                class="flex items-center gap-3"
              >
                <Skeleton class="size-10 shrink-0 bg-panel" />
                <div class="flex-1 space-y-1.5">
                  <Skeleton class="h-3.5 w-1/3 bg-panel" />
                  <Skeleton class="h-3 w-3/4 bg-panel" />
                </div>
              </div>
            </div>

            <ul
              v-else-if="hits.length"
              class="divide-y divide-line overflow-hidden rounded-md bg-panel ring-1 ring-card-line ring-inset"
            >
              <KitListItem
                v-for="hit in hits"
                :key="hit.projectId"
                class="rounded-none px-3.5"
                @select="choose(hit)"
              >
                <template #leading>
                  <KitThumb
                    :src="hit.iconUrl"
                    :alt="hit.title"
                    size="sm"
                  />
                </template>

                <div class="flex min-w-0 items-baseline gap-2">
                  <span class="truncate font-medium text-fg">{{ hit.title }}</span>
                  <span
                    v-if="hit.author"
                    class="shrink-0 text-caption text-fg-faint"
                  >{{ hit.author }}</span>
                  <Badge
                    v-if="installed.has(hit.projectId)"
                    class="shrink-0 self-center"
                  >
                    {{ $t('mod_catalog.installed') }}
                  </Badge>
                </div>
                <p class="truncate text-fg-muted">
                  {{ hit.description }}
                </p>

                <template #trailing>
                  <span class="flex shrink-0 items-center gap-1.5 text-caption text-fg-faint tabular-nums">
                    <Icon
                      name="i-lucide-download"
                      class="size-3.5"
                    />
                    {{ downloads(hit.downloads) }}
                  </span>
                  <Icon
                    name="i-lucide-chevron-right"
                    class="size-4 shrink-0 text-fg-faint"
                  />
                </template>
              </KitListItem>
            </ul>

            <KitEmpty
              v-else
              icon="i-lucide-search-x"
            >
              {{ $t('mod_catalog.nothing_found') }}
            </KitEmpty>

            <div
              v-if="hits.length < total"
              class="flex justify-center"
            >
              <Button
                :loading="searching"
                @click="search(hits.length)"
              >
                {{ $t('mod_catalog.show_more') }}
              </Button>
            </div>
          </template>
        </template>

        <template v-else-if="stage === 'versions'">
          <div
            v-if="loadingVersions"
            class="space-y-3 py-1"
          >
            <Skeleton
              v-for="row in 4"
              :key="row"
              class="h-10 bg-panel"
            />
          </div>

          <ul
            v-else-if="versions.length"
            class="divide-y divide-line overflow-hidden rounded-md bg-panel ring-1 ring-card-line ring-inset"
          >
            <li
              v-for="version in versions"
              :key="version.versionId"
              class="flex items-center gap-3 px-3.5 py-2.5"
            >
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="truncate font-mono text-body text-fg">{{ version.versionNumber }}</span>
                  <Badge :variant="RELEASE_TONE[version.release] ?? 'neutral'">
                    {{ RELEASE_KEYS[version.release] ? $t(RELEASE_KEYS[version.release]!) : version.release }}
                  </Badge>
                </div>
                <p class="text-caption text-fg-faint">
                  {{ [version.size ? modSize(version.size) : '', version.date ? new Date(version.date).toLocaleDateString(locale) : ''].filter(Boolean).join(' · ') }}
                </p>
              </div>

              <Badge
                v-if="version.blocked"
                variant="warning"
              >
                {{ $t('mod_catalog.blocked_version') }}
              </Badge>
              <Button
                v-else
                icon="i-lucide-download"
                :loading="planning"
                @click="pickVersion(version)"
              >
                {{ $t('mod_catalog.choose') }}
              </Button>
            </li>
          </ul>

          <KitEmpty
            v-else
            icon="i-lucide-package-x"
          >
            {{ $t('mod_catalog.no_versions') }}
          </KitEmpty>
        </template>

        <template v-else-if="plan">
          <div class="flex items-center gap-3 rounded-md bg-panel px-3.5 py-2.5 ring-2 ring-accent ring-inset">
            <KitThumb
              :src="plan.target.iconUrl"
              :alt="plan.target.title"
              size="sm"
            />
            <div class="min-w-0 flex-1">
              <p class="truncate font-medium text-fg">
                {{ plan.target.title }}
              </p>
              <p class="truncate font-mono text-caption text-fg-faint">
                {{ plan.target.versionNumber }}
              </p>
            </div>
            <Badge variant="accent">
              {{ $t('mod_catalog.installing_label') }}
            </Badge>
          </div>

          <section v-if="plan.required.length">
            <Label class="mb-1.5">
              {{ $t('mod_catalog.required') }}
            </Label>
            <ul class="divide-y divide-line overflow-hidden rounded-md bg-panel ring-1 ring-card-line ring-inset">
              <li
                v-for="item in plan.required"
                :key="item.projectId"
                class="flex items-center gap-3 px-3.5 py-2.5"
              >
                <KitThumb
                  :src="item.iconUrl"
                  :alt="item.title"
                  size="sm"
                />
                <div class="min-w-0 flex-1">
                  <p class="truncate font-medium text-fg">
                    {{ item.title }}
                  </p>
                  <p class="truncate font-mono text-caption text-fg-faint">
                    {{ item.versionNumber }}
                  </p>
                </div>
              </li>
            </ul>
          </section>

          <section v-if="plan.optional.length">
            <Label class="mb-1.5">
              {{ $t('mod_catalog.optional') }}
            </Label>
            <ul class="divide-y divide-line overflow-hidden rounded-md bg-panel ring-1 ring-card-line ring-inset">
              <KitListItem
                v-for="item in plan.optional"
                :key="item.projectId"
                class="rounded-none px-3.5"
                @select="toggleOptional(item.projectId)"
              >
                <template #leading>
                  <Checkbox
                    :model-value="optional.includes(item.projectId)"
                    tabindex="-1"
                    aria-hidden="true"
                    class="pointer-events-none"
                  />
                  <KitThumb
                    :src="item.iconUrl"
                    :alt="item.title"
                    size="sm"
                  />
                </template>
                <p class="truncate font-medium text-fg">
                  {{ item.title }}
                </p>
                <p class="truncate font-mono text-caption text-fg-faint">
                  {{ item.versionNumber }}
                </p>
              </KitListItem>
            </ul>
          </section>

          <KitNote
            v-if="plan.installed.length"
            icon="i-lucide-check"
            tone="success"
          >
            {{ $t('mod_catalog.already', { count: plan.installed.length }) }} · {{ plan.installed.join(', ') }}
          </KitNote>

          <Alert
            v-if="plan.target.blocked"
            variant="warning"
            icon="i-lucide-hand"
          >
            {{ $t('mod_catalog.blocked_hint') }}
            <template #action>
              <Button
                size="sm"
                icon="i-lucide-external-link"
                @click="openPage(plan.target.pageUrl)"
              >
                {{ $t('mod_catalog.open_page') }}
              </Button>
            </template>
          </Alert>
        </template>
      </DialogBody>

      <DialogFooter
        v-if="stage === 'plan' && plan"
        class="border-t border-line pt-3.5 pb-3.5"
      >
        <DialogClose as-child>
          <Button
            variant="ghost"
            :disabled="installing"
          >
            {{ $t('mod_catalog.close') }}
          </Button>
        </DialogClose>
        <Button
          v-if="!plan.target.blocked"
          variant="primary"
          icon="i-lucide-package-plus"
          :loading="installing"
          @click="install"
        >
          {{ $t('mod_catalog.install_count', installCount) }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
