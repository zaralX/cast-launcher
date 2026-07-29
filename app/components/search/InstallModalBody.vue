<script setup lang="ts">
import {v4} from "uuid";
import type {PackHit, PackVersion} from "~/types/catalog";
import {unsupportedReason, versionLabel} from "~/types/catalog";
import {INSTANCE_TYPE_LABELS, PACK_PROVIDER_LABELS} from "~/types/instance";
import {call} from "~/types/backend";
import {LauncherError} from "~/types/error";

const props = defineProps<{ hit: PackHit }>()

const emit = defineEmits<{ installed: [instanceId: string] }>()

const instanceStore = useInstanceStore()

const loading = ref(true)
const loadError = ref<LauncherError | null>(null)
const creating = ref(false)

const versions = ref<PackVersion[]>([])
const versionId = ref("")

const name = ref(props.hit.title)
const description = ref(props.hit.description)

const selected = computed(() => versions.value.find(version => version.id === versionId.value) ?? null)

const versionItems = computed(() => versions.value.map(version => ({
  label: versionLabel(version),
  value: version.id,
  disabled: !version.supported
})))

const loaderLabel = computed(() => {
  const loader = selected.value?.loader
  return loader ? INSTANCE_TYPE_LABELS[loader] : "неизвестен"
})

const canInstall = computed(() =>
    !loading.value
    && !loadError.value
    && !creating.value
    && name.value.trim().length > 0
    && !!selected.value?.supported
)

async function loadVersions() {
  loading.value = true
  loadError.value = null

  try {
    versions.value = await call("list_pack_versions", {
      provider: props.hit.provider,
      projectId: props.hit.projectId
    })
    versionId.value = (versions.value.find(version => version.supported) ?? versions.value[0])?.id ?? ""
  } catch (e) {
    loadError.value = captureError(e, {code: "NETWORK", context: {action: "Загрузка версий модпака"}})
  } finally {
    loading.value = false
  }
}

onMounted(loadVersions)

async function packIcon(): Promise<string | undefined> {
  const url = props.hit.iconUrl
  if (!url) return undefined

  const saved = await safeRun(() => call("save_pack_icon", {
    provider: props.hit.provider,
    projectId: props.hit.projectId,
    url
  }))

  return saved?.name
}

const install = async () => {
  if (!canInstall.value) return

  const version = selected.value
  const file = version?.file

  if (!version?.loader || !version.minecraftVersion || !file) return

  creating.value = true

  const icon = await packIcon()

  const created = await attempt(() => instanceStore.createInstance({
    id: v4(),
    name: name.value.trim(),
    description: description.value.trim(),
    type: version.loader!,
    minecraftVersion: version.minecraftVersion!,
    icon,
    version: 1,
    pack: {
      provider: props.hit.provider,
      projectId: props.hit.projectId,
      versionId: version.id,
      versionNumber: version.versionNumber || version.name,
      fileUrl: file.url,
      fileName: file.filename,
      fileSha1: file.hashes.sha1 ?? undefined,
      fileSize: file.size ?? undefined
    }
  }))

  if (!created.ok) {
    creating.value = false
    return
  }

  await safeRun(() => instanceStore.installInstance(created.value.id), {
    code: "NETWORK",
    context: {action: "Установка модпака", instanceName: created.value.name}
  })

  creating.value = false
  emit("installed", created.value.id)
}
</script>

<template>
  <div>
    <div class="flex items-start gap-4 border-b border-line pb-5">
      <span class="grid size-12 shrink-0 place-items-center overflow-hidden border border-line bg-ink-900">
        <img v-if="hit.iconUrl" :src="hit.iconUrl" :alt="hit.title" class="size-full object-cover"/>
        <UIcon v-else name="i-lucide-package" class="size-5 text-fg-faint"/>
      </span>

      <div class="min-w-0">
        <p class="font-mono text-[9px] uppercase tracking-[0.24em] text-acid">
          {{ PACK_PROVIDER_LABELS[hit.provider] }}
        </p>
        <h3 class="mt-1.5 truncate font-unbounded text-[15px] font-semibold tracking-[-0.04em] text-fg">
          {{ hit.title }}
        </h3>
      </div>
    </div>

    <div v-if="loading" class="flex flex-col items-center gap-4 py-14">
      <span class="relative block h-px w-40 overflow-hidden bg-line">
        <span class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"/>
      </span>
      <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Загрузка версий пака</p>
    </div>

    <div v-else-if="loadError" class="mt-6 border border-red-400/30 bg-ink-900 p-5">
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

    <p v-else-if="!versions.length" class="mt-6 text-[12px] leading-relaxed text-fg-muted">
      У этого модпака нет опубликованных версий.
    </p>

    <form v-else class="mt-6 space-y-8" @submit.prevent="install">
      <div class="space-y-5">
        <div>
          <label for="pack-name" class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            Название
          </label>
          <UInput
              id="pack-name"
              v-model="name"
              size="lg"
              class="w-full"
              :ui="{ base: 'font-unbounded text-[15px] tracking-[-0.03em]' }"
          />
        </div>

        <div>
          <label
              for="pack-description"
              class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            Описание
          </label>
          <UInput id="pack-description" v-model="description" placeholder="Необязательно" class="w-full"/>
        </div>

        <div>
          <label class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Версия пака</label>
          <USelectMenu
              v-model="versionId"
              :items="versionItems"
              value-key="value"
              :search-input="{ placeholder: 'Версия или Minecraft' }"
              class="w-full"
          />
        </div>
      </div>

      <div class="grid grid-cols-2 border border-line">
        <div class="px-4 py-3">
          <p class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Minecraft</p>
          <p class="mt-1.5 font-unbounded text-[13px] tracking-[-0.03em] text-fg">
            {{ selected?.minecraftVersion ?? "—" }}
          </p>
        </div>
        <div class="border-l border-line px-4 py-3">
          <p class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">Загрузчик</p>
          <p class="mt-1.5 font-unbounded text-[13px] tracking-[-0.03em] text-fg">{{ loaderLabel }}</p>
        </div>
      </div>

      <p v-if="selected && !selected.supported" class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted">
        <UIcon name="i-lucide-triangle-alert" class="mt-0.5 size-3.5 shrink-0 text-amber-400"/>
        Эту версию лаунчер установить не сможет: {{ unsupportedReason(selected) }}. Выберите другую версию.
      </p>

      <p v-else-if="selected?.blocked" class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted">
        <UIcon name="i-lucide-hand" class="mt-0.5 size-3.5 shrink-0 text-amber-400"/>
        Автор запретил сторонним лаунчерам скачивать архив этого пака. Установка начнётся и остановится
        на окне со ссылкой: скачайте архив сами и укажите папку — дальше лаунчер справится.
      </p>

      <p v-else-if="!hit.distributionAllowed" class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted">
        <UIcon name="i-lucide-triangle-alert" class="mt-0.5 size-3.5 shrink-0 text-amber-400"/>
        Автор запретил сторонним лаунчерам раздавать файлы этого пака. Лаунчер поищет замену на Modrinth,
        а то, что не найдётся, попросит скачать вручную — со ссылками и поиском по папке загрузок.
      </p>

      <AppButton
          block
          type="submit"
          class="group/act h-11 tracking-[0.2em]"
          :loading="creating"
          :disabled="!canInstall"
      >
        <template #leading>
          <UIcon
              name="i-lucide-download"
              class="size-3.5 transition-transform duration-500 group-hover/act:translate-y-0.5"
          />
        </template>
        {{ creating ? 'Создание сборки' : 'Установить' }}
      </AppButton>
    </form>
  </div>
</template>
