<script setup lang="ts">
import type {Instance} from "~/types/instance";
import {INSTANCE_TYPE_LABELS, PACK_PROVIDER_LABELS} from "~/types/instance";
import type {BlockedFile, PackVersion} from "~/types/catalog";
import {unsupportedReason, versionLabel} from "~/types/catalog";
import {call} from "~/types/backend";
import {LauncherError} from "~/types/error";

const props = defineProps<{ instance: Instance }>()

const instanceStore = useInstanceStore()
const toast = useToast()

const loading = ref(true)
const loadError = ref<LauncherError | null>(null)
const updating = ref(false)

const versions = ref<PackVersion[]>([])
const versionId = ref("")

const blockedFiles = ref<BlockedFile[]>([])

const pack = computed(() => props.instance.pack)
const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const selected = computed(() => versions.value.find(version => version.id === versionId.value) ?? null)

const versionItems = computed(() => versions.value.map(version => ({
  label: versionLabel(version),
  value: version.id,
  disabled: !version.supported
})))

const latest = computed(() => versions.value.find(version => version.supported) ?? null)

const updateAvailable = computed(() =>
    !!latest.value && !!pack.value && latest.value.id !== pack.value.versionId
)

const changed = computed(() => !!pack.value && versionId.value !== pack.value.versionId)

const blocked = computed(() => {
  if (running.value) return "Сборка запущена — сначала закройте игру"
  if (installing.value) return "Дождитесь окончания текущей установки"
  return null
})

const canApply = computed(() =>
    !loading.value && !loadError.value && !updating.value && !blocked.value && !!selected.value?.supported
)

const facts = computed(() => [
  {label: "Источник", value: pack.value ? PACK_PROVIDER_LABELS[pack.value.provider] : "—"},
  {label: "Проект", value: pack.value?.projectId ?? "—"},
  {label: "Текущая версия", value: pack.value?.versionNumber || pack.value?.versionId || "—"},
  {label: "Архив пака", value: pack.value?.fileName || "—"}
])

async function loadVersions() {
  if (!pack.value) return

  loading.value = true
  loadError.value = null

  try {
    versions.value = await call("list_pack_versions", {
      provider: pack.value.provider,
      projectId: pack.value.projectId
    })
  } catch (e) {
    loadError.value = captureError(e, {
      code: "NETWORK",
      context: {instanceId: props.instance.id, action: "Загрузка версий модпака"}
    })
  } finally {
    loading.value = false
  }
}

async function loadBlocked() {
  blockedFiles.value = await safeRun(() => call("list_pack_blocked", {instanceId: props.instance.id})) ?? []
}

onMounted(() => {
  loadVersions()
  loadBlocked()
})

watch(installing, running => {
  if (!running) loadBlocked()
})

watch(() => pack.value?.versionId, id => {
  versionId.value = id ?? ""
}, {immediate: true})

const selectLatest = () => {
  if (latest.value) versionId.value = latest.value.id
}

const openPage = (url: string) => safeRun(() => call("open_url", {url}))

const openMods = () => safeRun(() => call("open_instance_dir", {
  instanceId: props.instance.id,
  target: "minecraft"
}))

async function apply() {
  if (!canApply.value || !changed.value) return

  updating.value = true

  const context = {instanceId: props.instance.id, action: "Смена версии модпака"}

  const switched = await attempt(
      () => call("set_instance_pack_version", {instanceId: props.instance.id, versionId: versionId.value}),
      {context}
  )

  if (!switched.ok) {
    updating.value = false
    return
  }

  await safeRun(() => instanceStore.installInstance(props.instance.id), {code: "NETWORK", context})

  updating.value = false

  toast.add({
    title: `Обновление до ${switched.value.pack?.versionNumber ?? "новой версии"}`,
    description: "Файлы пака докачиваются в фоне",
    color: "success",
    icon: "i-lucide-refresh-cw"
  })
}
</script>

<template>
  <div class="space-y-6">
    <SettingsPanel
        index="01"
        title="Версия модпака"
        description="Смена версии как в Prism: выберите нужную и нажмите «Обновить»."
        icon="i-lucide-package"
    >
      <div v-if="!pack" class="text-[12px] leading-relaxed text-fg-muted">
        Эта сборка создана вручную, у неё нет версий пака.
      </div>

      <div v-else-if="loading" class="flex flex-col items-center gap-4 py-10">
        <span class="relative block h-px w-40 overflow-hidden bg-line">
          <span class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"/>
        </span>
        <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Загрузка версий пака</p>
      </div>

      <div v-else-if="loadError" class="border border-red-400/30 bg-ink-900 p-5">
        <div class="flex items-start gap-3">
          <UIcon name="i-lucide-wifi-off" class="mt-0.5 size-4 shrink-0 text-red-400"/>
          <div class="min-w-0">
            <p class="text-[13px] font-medium text-fg">{{ loadError.title }}</p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">{{ loadError.hint ?? loadError.message }}</p>
            <AppButton
                tone="quiet"
                class="mt-4 text-[10px] tracking-[0.18em]"
                icon="i-lucide-rotate-cw"
                @click="loadVersions"
            >
              Повторить
            </AppButton>
          </div>
        </div>
      </div>

      <div v-else class="space-y-7">
        <div
            v-if="updateAvailable"
            class="flex items-center justify-between gap-4 border border-acid/30 bg-acid/[0.04] px-4 py-3"
        >
          <p class="min-w-0 text-[12px] leading-relaxed text-fg-muted">
            Доступна новая версия:
            <span class="text-fg">{{ latest?.versionNumber || latest?.name }}</span>
          </p>

          <AppButton tone="quiet" class="shrink-0 text-[10px] tracking-[0.18em]" @click="selectLatest">
            Выбрать
          </AppButton>
        </div>

        <SettingsField label="Версия пака">
          <USelectMenu
              v-model="versionId"
              :items="versionItems"
              value-key="value"
              :search-input="{ placeholder: 'Версия или Minecraft' }"
              class="w-full"
          />
        </SettingsField>

        <div class="grid grid-cols-2 border border-line">
          <div class="px-4 py-3">
            <p class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Minecraft</p>
            <p class="mt-1.5 font-unbounded text-[13px] tracking-[-0.03em] text-fg">
              {{ selected?.minecraftVersion ?? instance.minecraftVersion }}
            </p>
          </div>
          <div class="border-l border-line px-4 py-3">
            <p class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Загрузчик</p>
            <p class="mt-1.5 font-unbounded text-[13px] tracking-[-0.03em] text-fg">
              {{ selected?.loader ? INSTANCE_TYPE_LABELS[selected.loader] : INSTANCE_TYPE_LABELS[instance.type] }}
            </p>
          </div>
        </div>

        <p
            v-if="selected && !selected.supported"
            class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
        >
          <UIcon name="i-lucide-triangle-alert" class="mt-0.5 size-3.5 shrink-0 text-amber-400"/>
          Эту версию лаунчер установить не сможет: {{ unsupportedReason(selected) }}.
        </p>

        <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
          <p class="min-w-0 text-[12px] leading-relaxed text-fg-muted">
            Моды, конфиги и ресурспаки пака будут заменены на файлы выбранной версии, а то, что осталось от прошлой,
            лаунчер уберёт. Миры, скриншоты и всё, что вы добавили сами, останутся на месте.
          </p>

          <AppButton
              class="h-9 shrink-0 px-3.5 text-[10px] tracking-[0.18em]"
              icon="i-lucide-refresh-cw"
              :loading="updating"
              :disabled="!canApply || !changed"
              @click="apply"
          >
            {{ updating ? 'Обновление' : 'Обновить' }}
          </AppButton>
        </div>

        <p v-if="blocked" class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
          {{ blocked }}
        </p>
        <p
            v-else-if="!changed"
            class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint"
        >
          Выбрана текущая версия
        </p>
      </div>
    </SettingsPanel>

    <SettingsPanel
        v-if="blockedFiles.length"
        index="02"
        title="Скачать вручную"
        description="Автор этих модов запретил сторонним лаунчерам их раздавать."
        icon="i-lucide-hand"
    >
      <p class="text-[12px] leading-relaxed text-fg-muted">
        Замену на Modrinth лаунчер не нашёл. Откройте страницу каждого файла, скачайте его и положите
        в папку сборки по указанному пути — после этого пак заработает полностью.
      </p>

      <ul class="mt-5 divide-y divide-line border border-line">
        <li v-for="file in blockedFiles" :key="file.targetPath" class="flex items-center gap-4 px-4 py-3">
          <div class="min-w-0 flex-1">
            <p class="truncate text-[12px] text-fg" :title="file.fileName">{{ file.fileName }}</p>
            <p class="mt-1 truncate font-mono text-[10px] text-fg-faint" :title="file.targetPath">
              {{ file.targetPath }}
            </p>
          </div>

          <AppButton
              v-if="file.websiteUrl"
              tone="quiet"
              class="shrink-0 text-[10px] tracking-[0.16em]"
              icon="i-lucide-external-link"
              @click="openPage(file.websiteUrl)"
          >
            Открыть
          </AppButton>
        </li>
      </ul>

      <AppButton
          tone="quiet"
          class="mt-5 text-[10px] tracking-[0.16em]"
          icon="i-lucide-folder-open"
          @click="openMods"
      >
        Открыть папку игры
      </AppButton>
    </SettingsPanel>

    <SettingsPanel
        :index="blockedFiles.length ? '03' : '02'"
        title="Состав пака"
        description="Что лаунчер считает файлами модпака."
        icon="i-lucide-list"
    >
      <dl class="grid gap-x-6 gap-y-4 sm:grid-cols-2">
        <div v-for="fact in facts" :key="fact.label" class="min-w-0">
          <dt class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ fact.label }}</dt>
          <dd class="mt-1.5 truncate font-mono text-[12px] text-fg-muted" :title="fact.value">{{ fact.value }}</dd>
        </div>
      </dl>

      <p class="mt-6 border-t border-line pt-6 text-[12px] leading-relaxed text-fg-muted">
        Список файлов установленной версии лежит в <span class="font-mono text-fg-faint">pack-files.json</span>
        в папке сборки. По нему при обновлении убирается ровно то, что положил пак, — остальное не трогается.
      </p>
    </SettingsPanel>
  </div>
</template>
