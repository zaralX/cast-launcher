<script setup lang="ts">
import type {Instance} from "~/types/instance"
import type {PackHit, PackProviderInfo} from "~/types/catalog"
import type {CatalogVersion, InstallPlan} from "~/types/mods"
import {RELEASE_LABELS, modSize} from "~/types/mods"
import {call} from "~/types/backend"

const props = defineProps<{ instance: Instance, open: boolean }>()
const emit = defineEmits<{ "update:open": [boolean], installed: [] }>()

type Provider = "modrinth" | "curseforge"

const PAGE = 20
const DEBOUNCE = 350

const SORTS = [
  {label: "По совпадению", value: "relevance"},
  {label: "По загрузкам", value: "downloads"},
  {label: "По свежести", value: "updated"}
]

const modsStore = useModsStore()
const toast = useToast()

const instanceId = computed(() => props.instance.id)

const providers = ref<PackProviderInfo[]>([])
const provider = ref<Provider>("modrinth")
const query = ref("")
const sort = ref("relevance")

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
  if (plan.value) return "plan"
  if (chosen.value) return "versions"
  return "search"
})

const title = computed(() => {
  if (stage.value === "plan") return "Что будет установлено"
  if (stage.value === "versions") return chosen.value?.title ?? "Версии"
  return "Найти мод"
})

let timer: ReturnType<typeof setTimeout> | null = null

function close() {
  emit("update:open", false)
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
        limit: PAGE
      }),
      {context: {instanceId: instanceId.value, action: "Поиск модов"}}
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
      {context: {instanceId: instanceId.value, action: "Версии мода"}}
  )

  loadingVersions.value = false

  if (result.ok) versions.value = result.value
}

async function pickVersion(version: CatalogVersion) {
  if (!chosen.value) return

  planning.value = true

  const result = await attempt(
      () => modsStore.planInstall(instanceId.value, provider.value, chosen.value!.projectId, version.versionId),
      {context: {instanceId: instanceId.value, action: "Разбор зависимостей"}}
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
      {context: {instanceId: instanceId.value, action: "Установка мода"}}
  )

  installing.value = false

  if (!result.ok) return

  const {installed: added, failed, blocked} = result.value

  toast.add({
    title: added.length ? `Установлено: ${added.length}` : "Ничего не установлено",
    description: [
      failed.length ? `не удалось: ${failed.join(", ")}` : "",
      blocked.length ? `качать вручную: ${blocked.join(", ")}` : ""
    ].filter(Boolean).join("; ") || undefined,
    color: failed.length || blocked.length ? "warning" : "success",
    icon: "i-lucide-package-plus"
  })

  emit("installed")

  plan.value = null
  optional.value = []
  chosen.value = null
}

const openPage = (url: string) => safeRun(
    () => call("open_url", {url}),
    {context: {instanceId: instanceId.value, action: "Открытие страницы мода"}}
)

function downloads(value: number) {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${Math.round(value / 1_000)}K`

  return String(value)
}

watch(() => props.open, async (open) => {
  if (!open) return

  if (!providers.value.length) {
    const result = await attempt(() => call("pack_providers"), {context: {action: "Каталоги"}})
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
  <UModal
      :open="open"
      :title="title"
      :ui="{ content: 'max-w-3xl' }"
      @update:open="value => emit('update:open', value)"
  >
    <template #body>
      <div class="space-y-5">
        <div v-if="instance.type === 'vanilla'" class="flex items-center gap-2.5 border border-amber-400/40 px-4 py-3">
          <span class="size-1.5 shrink-0 bg-amber-400 animate-blink"/>
          <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-amber-400">
            В сборке нет загрузчика модов - ставить моды некуда
          </p>
        </div>

        <template v-if="stage === 'search'">
          <div class="flex flex-wrap items-end gap-4">
            <SettingsField label="Поиск" class="min-w-[14rem] flex-1">
              <UInput
                  v-model="query"
                  placeholder="Название мода"
                  class="w-full"
                  @update:model-value="searchLater"
                  @keydown.enter="search()"
              >
                <template #trailing>
                  <UIcon name="i-lucide-search" class="size-3.5 text-fg-faint"/>
                </template>
              </UInput>
            </SettingsField>

            <SettingsField label="Каталог" class="min-w-[9rem]">
              <USelect
                  v-model="provider"
                  :items="[
                    {label: 'Modrinth', value: 'modrinth'},
                    {label: 'CurseForge', value: 'curseforge'}
                  ]"
                  class="w-full"
              />
            </SettingsField>

            <SettingsField label="Сортировка" class="min-w-[10rem]">
              <USelect v-model="sort" :items="SORTS" class="w-full"/>
            </SettingsField>
          </div>

          <p
              v-if="!providerReady"
              class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-amber-400"
          >
            <span class="size-1.5 bg-amber-400 animate-blink"/>
            {{ providerInfo?.reason ?? 'Каталог недоступен' }}
          </p>

          <div v-else class="max-h-[26rem] space-y-px overflow-auto border-t border-line">
            <button
                v-for="hit in hits"
                :key="hit.projectId"
                type="button"
                class="flex w-full items-center gap-4 border-b border-line px-1 py-3 text-left transition-colors duration-300 hover:bg-ink-700"
                @click="choose(hit)"
            >
              <span class="grid size-10 shrink-0 place-items-center overflow-hidden border border-line bg-ink-900">
                <img v-if="hit.iconUrl" :src="hit.iconUrl" alt="" class="size-full object-cover"/>
                <span v-else class="font-mono text-[10px] text-fg-faint">{{ hit.title.slice(0, 2).toUpperCase() }}</span>
              </span>

              <span class="min-w-0 flex-1">
                <span class="flex items-center gap-2.5">
                  <span class="truncate text-[13px] font-medium text-fg">{{ hit.title }}</span>

                  <span
                      v-if="installed.has(hit.projectId)"
                      class="shrink-0 border border-line px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint"
                  >
                    установлен
                  </span>
                </span>

                <span class="mt-1 block truncate text-[12px] text-fg-muted">{{ hit.description }}</span>
              </span>

              <span class="hidden shrink-0 flex-col items-end gap-1 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint sm:flex">
                <span>{{ downloads(hit.downloads) }}</span>
                <span v-if="hit.author" class="max-w-[8rem] truncate">{{ hit.author }}</span>
              </span>
            </button>

            <p v-if="!hits.length && !searching" class="py-10 text-center font-mono text-[10px] uppercase tracking-[0.22em] text-fg-faint">
              Ничего не нашлось
            </p>

            <div v-if="hits.length < total" class="py-4 text-center">
              <AppButton
                  tone="quiet"
                  class="text-[10px] tracking-[0.18em]"
                  :loading="searching"
                  @click="search(hits.length)"
              >
                Показать ещё
              </AppButton>
            </div>
          </div>
        </template>

        <template v-else-if="stage === 'versions'">
          <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
            Совместимо с {{ instance.minecraftVersion }}<template v-if="instance.type !== 'vanilla'"> · {{ instance.type }}</template>
          </p>

          <div class="max-h-[24rem] overflow-auto border-t border-line">
            <div
                v-for="version in versions"
                :key="version.versionId"
                class="flex items-center gap-4 border-b border-line px-1 py-3"
            >
              <span class="min-w-0 flex-1">
                <span class="block truncate text-[13px] text-fg">{{ version.versionNumber }}</span>
                <span class="mt-1 block font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">
                  {{ RELEASE_LABELS[version.release] ?? version.release }}
                  <template v-if="version.size"> · {{ modSize(version.size) }}</template>
                  <template v-if="version.date"> · {{ new Date(version.date).toLocaleDateString("ru-RU") }}</template>
                </span>
              </span>

              <span
                  v-if="version.blocked"
                  class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-amber-400"
              >
                качать с сайта
              </span>

              <AppButton
                  v-else
                  class="h-8 px-3 text-[10px] tracking-[0.18em]"
                  icon="i-lucide-download"
                  :loading="planning"
                  @click="pickVersion(version)"
              >
                Выбрать
              </AppButton>
            </div>

            <p
                v-if="!versions.length && !loadingVersions"
                class="py-10 text-center font-mono text-[10px] uppercase tracking-[0.22em] text-fg-faint"
            >
              Совместимых версий нет
            </p>
          </div>
        </template>

        <template v-else-if="plan">
          <div class="space-y-4 border-t border-line pt-4">
            <div class="flex items-center gap-3">
              <span class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">Ставим</span>
              <span class="text-[13px] text-fg">{{ plan.target.title }}</span>
              <span class="font-mono text-[10px] text-fg-faint">{{ plan.target.versionNumber }}</span>
            </div>

            <div v-if="plan.required.length" class="space-y-2">
              <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">Нужны для работы</p>
              <p v-for="item in plan.required" :key="item.projectId" class="text-[12px] text-fg-muted">
                {{ item.title }} · {{ item.versionNumber }}
              </p>
            </div>

            <div v-if="plan.optional.length" class="space-y-2">
              <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">Можно добавить</p>
              <label
                  v-for="item in plan.optional"
                  :key="item.projectId"
                  class="flex cursor-pointer items-center gap-2.5"
              >
                <UCheckbox
                    :model-value="optional.includes(item.projectId)"
                    @update:model-value="toggleOptional(item.projectId)"
                />
                <span class="text-[12px] text-fg-muted">{{ item.title }} · {{ item.versionNumber }}</span>
              </label>
            </div>

            <p v-if="plan.installed.length" class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
              Уже в сборке: {{ plan.installed.length }}
            </p>

            <div
                v-if="plan.target.blocked"
                class="space-y-3 border border-amber-400/40 px-4 py-3"
            >
              <p class="font-mono text-[10px] uppercase leading-relaxed tracking-[0.2em] text-amber-400">
                CurseForge запретил скачивание этого файла мимо сайта
              </p>

              <AppButton
                  tone="quiet"
                  class="text-[10px] tracking-[0.18em]"
                  icon="i-lucide-external-link"
                  @click="openPage(plan.target.pageUrl)"
              >
                Открыть страницу мода
              </AppButton>
            </div>
          </div>
        </template>

        <div class="flex items-center justify-between gap-4 border-t border-line pt-5">
          <AppButton
              v-if="stage !== 'search'"
              tone="quiet"
              class="text-[10px] tracking-[0.16em]"
              icon="i-lucide-arrow-left"
              :disabled="installing"
              @click="back"
          >
            Назад
          </AppButton>

          <span v-else class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
            <template v-if="searching">Ищем</template>
            <template v-else-if="total">Найдено: {{ total }}</template>
          </span>

          <div class="flex items-center gap-4">
            <AppButton
                tone="quiet"
                class="text-[10px] tracking-[0.16em]"
                :disabled="installing"
                @click="close"
            >
              Закрыть
            </AppButton>

            <AppButton
                v-if="stage === 'plan' && plan && !plan.target.blocked"
                class="h-10 px-6 tracking-[0.18em]"
                icon="i-lucide-package-plus"
                :loading="installing"
                @click="install"
            >
              Установить
            </AppButton>
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>
