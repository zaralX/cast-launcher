<script setup lang="ts">
import type { Instance } from '~/types/instance'
import type { PackHit, PackProviderInfo } from '~/types/catalog'
import type { CatalogVersion, InstallPlan } from '~/types/mods'
import { RELEASE_KEYS } from '~/types/mods'
import type { SelectOption } from '~/types/ui'

const props = defineProps<{ instance: Instance, open: boolean }>()
const emit = defineEmits<{ 'update:open': [boolean], 'installed': [] }>()

type Provider = 'modrinth' | 'curseforge'

const PAGE = 20
const DEBOUNCE = 350

const PROVIDERS: SelectOption<Provider>[] = [
  { label: 'Modrinth', value: 'modrinth' },
  { label: 'CurseForge', value: 'curseforge' },
]

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

let timer: ReturnType<typeof setTimeout> | null = null

function close() {
  emit('update:open', false)
}

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
    <DialogContent class="max-w-3xl">
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
      </DialogHeader>

      <DialogBody class="space-y-5">
        <Alert
          v-if="instance.type === 'vanilla'"
          variant="warning"
          class="bg-transparent"
        >
          <KitStatus tone="warning">
            {{ $t('mod_catalog.no_loader') }}
          </KitStatus>
        </Alert>

        <template v-if="stage === 'search'">
          <div class="flex flex-wrap items-end gap-4">
            <KitField
              :label="$t('mod_catalog.search')"
              class="min-w-56 flex-1"
            >
              <KitSearchInput
                v-model="query"
                :loading="searching"
                :placeholder="$t('mod_catalog.search_placeholder')"
                @update:model-value="searchLater"
                @keydown.enter="search()"
              />
            </KitField>

            <KitField
              :label="$t('mod_catalog.provider')"
              class="min-w-36"
            >
              <KitSelect
                v-model="provider"
                :items="PROVIDERS"
              />
            </KitField>

            <KitField
              :label="$t('mod_catalog.sort')"
              class="min-w-40"
            >
              <KitSelect
                v-model="sort"
                :items="SORTS"
              />
            </KitField>
          </div>

          <KitStatus
            v-if="!providerReady"
            tone="warning"
          >
            {{ providerInfo?.reason ?? $t('mod_catalog.unavailable') }}
          </KitStatus>

          <div
            v-else
            class="max-h-[26rem] space-y-px overflow-auto border-t border-line"
          >
            <button
              v-for="hit in hits"
              :key="hit.projectId"
              type="button"
              class="flex w-full cursor-pointer items-center gap-4 border-b border-line px-1 py-3 text-left outline-none transition-colors duration-300 hover:bg-ink-700 focus-visible:bg-ink-700"
              @click="choose(hit)"
            >
              <KitThumb
                :src="hit.iconUrl"
                size="sm"
              >
                <span class="font-mono text-label text-fg-faint">{{ hit.title.slice(0, 2).toUpperCase() }}</span>
              </KitThumb>

              <span class="min-w-0 flex-1">
                <span class="flex items-center gap-2.5">
                  <span class="truncate text-title font-medium text-fg">{{ hit.title }}</span>

                  <Badge
                    v-if="installed.has(hit.projectId)"
                    class="text-fg-faint"
                  >
                    {{ $t('mod_catalog.installed') }}
                  </Badge>
                </span>

                <span class="mt-1 block truncate text-body text-fg-muted">{{ hit.description }}</span>
              </span>

              <span class="flex shrink-0 flex-col items-end gap-1 font-mono text-micro uppercase tracking-caps text-fg-faint">
                <span>{{ downloads(hit.downloads) }}</span>
                <span
                  v-if="hit.author"
                  class="max-w-32 truncate"
                >{{ hit.author }}</span>
              </span>
            </button>

            <p
              v-if="!hits.length && !searching"
              class="py-10 text-center font-mono text-label uppercase tracking-caps text-fg-faint"
            >
              {{ $t('mod_catalog.nothing_found') }}
            </p>

            <div
              v-if="hits.length < total"
              class="py-4 text-center"
            >
              <Button
                variant="quiet"
                :loading="searching"
                @click="search(hits.length)"
              >
                {{ $t('mod_catalog.show_more') }}
              </Button>
            </div>
          </div>
        </template>

        <template v-else-if="stage === 'versions'">
          <p class="font-mono text-label uppercase tracking-caps text-fg-faint">
            {{ $t('mod_catalog.compatible', { version: instance.minecraftVersion }) }}<template v-if="instance.type !== 'vanilla'">
              · {{ instance.type }}
            </template>
          </p>

          <div class="max-h-[24rem] overflow-auto border-t border-line">
            <div
              v-for="version in versions"
              :key="version.versionId"
              class="flex items-center gap-4 border-b border-line px-1 py-3"
            >
              <span class="min-w-0 flex-1">
                <span class="block truncate text-title text-fg">{{ version.versionNumber }}</span>
                <span class="mt-1 block font-mono text-label uppercase tracking-caps text-fg-faint">
                  {{ RELEASE_KEYS[version.release] ? $t(RELEASE_KEYS[version.release]!) : version.release }}
                  <template v-if="version.size"> · {{ modSize(version.size) }}</template>
                  <template v-if="version.date"> · {{ new Date(version.date).toLocaleDateString(locale) }}</template>
                </span>
              </span>

              <Badge
                v-if="version.blocked"
                variant="warning"
                class="border-transparent"
              >
                {{ $t('mod_catalog.blocked_version') }}
              </Badge>

              <Button
                v-else
                size="sm"
                icon="i-lucide-download"
                :loading="planning"
                @click="pickVersion(version)"
              >
                {{ $t('mod_catalog.choose') }}
              </Button>
            </div>

            <p
              v-if="!versions.length && !loadingVersions"
              class="py-10 text-center font-mono text-label uppercase tracking-caps text-fg-faint"
            >
              {{ $t('mod_catalog.no_versions') }}
            </p>
          </div>
        </template>

        <div
          v-else-if="plan"
          class="space-y-4 border-t border-line pt-4"
        >
          <div class="flex items-center gap-3">
            <Label>{{ $t('mod_catalog.installing_label') }}</Label>
            <span class="text-title text-fg">{{ plan.target.title }}</span>
            <span class="font-mono text-label text-fg-faint">{{ plan.target.versionNumber }}</span>
          </div>

          <div
            v-if="plan.required.length"
            class="space-y-2"
          >
            <Label>{{ $t('mod_catalog.required') }}</Label>
            <p
              v-for="item in plan.required"
              :key="item.projectId"
              class="text-body text-fg-muted"
            >
              {{ item.title }} · {{ item.versionNumber }}
            </p>
          </div>

          <div
            v-if="plan.optional.length"
            class="space-y-2"
          >
            <Label>{{ $t('mod_catalog.optional') }}</Label>
            <label
              v-for="item in plan.optional"
              :key="item.projectId"
              class="flex cursor-pointer items-center gap-2.5"
            >
              <Checkbox
                :model-value="optional.includes(item.projectId)"
                @update:model-value="toggleOptional(item.projectId)"
              />
              <span class="text-body text-fg-muted">{{ item.title }} · {{ item.versionNumber }}</span>
            </label>
          </div>

          <p
            v-if="plan.installed.length"
            class="font-mono text-label uppercase tracking-caps text-fg-faint"
          >
            {{ $t('mod_catalog.already', { count: plan.installed.length }) }}
          </p>

          <Alert
            v-if="plan.target.blocked"
            variant="warning"
            class="bg-transparent"
          >
            <p class="font-mono text-label uppercase leading-relaxed tracking-caps text-warning">
              {{ $t('mod_catalog.blocked_hint') }}
            </p>

            <Button
              variant="quiet"
              icon="i-lucide-external-link"
              class="mt-1"
              @click="openPage(plan.target.pageUrl)"
            >
              {{ $t('mod_catalog.open_page') }}
            </Button>
          </Alert>
        </div>
      </DialogBody>

      <DialogFooter class="justify-between">
        <Button
          v-if="stage !== 'search'"
          variant="quiet"
          icon="i-lucide-arrow-left"
          :disabled="installing"
          @click="back"
        >
          {{ $t('mod_catalog.back') }}
        </Button>

        <span
          v-else
          class="font-mono text-label uppercase tracking-caps text-fg-faint"
        >
          <template v-if="searching">{{ $t('mod_catalog.searching') }}</template>
          <template v-else-if="total">{{ $t('mod_catalog.found', { count: total }) }}</template>
        </span>

        <div class="flex items-center gap-4">
          <Button
            variant="quiet"
            :disabled="installing"
            @click="close"
          >
            {{ $t('mod_catalog.close') }}
          </Button>

          <Button
            v-if="stage === 'plan' && plan && !plan.target.blocked"
            size="lg"
            icon="i-lucide-package-plus"
            :loading="installing"
            @click="install"
          >
            {{ $t('mod_catalog.install') }}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
