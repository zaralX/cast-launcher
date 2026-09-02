<script setup lang="ts">
import {storeToRefs} from "pinia"
import {call} from "~/types/backend"
import {useImportStore} from "~/stores/import"
import {formatPlaytime} from "~/types/instance"
import {
  defaultImportOptions,
  formatBytes,
  type DetectedLauncher,
  type ImportProgress,
  type ImportReport,
  type LauncherKind,
  type ScannedInstance
} from "~/types/import"

interface SourceInfo {
  kind: LauncherKind
  label: string
  hintKey: string
  icon: string
  placeholder: string
  librariesHintKey: string
}

const SOURCES: SourceInfo[] = [
  {
    kind: "prism",
    label: "PrismLauncher",
    hintKey: "settings.import.prism.hint",
    icon: "i-lucide-package",
    placeholder: "%APPDATA%\\PrismLauncher",
    librariesHintKey: "settings.import.prism.libraries_hint"
  },
  {
    kind: "modrinth",
    label: "Modrinth App",
    hintKey: "settings.import.modrinth.hint",
    icon: "i-lucide-box",
    placeholder: "%APPDATA%\\ModrinthApp",
    librariesHintKey: "settings.import.modrinth.libraries_hint"
  }
]

const STAGE_KEYS: Record<ImportProgress["stage"], string> = {
  shared: "settings.import.stage.shared",
  instances: "settings.import.stage.instances",
  done: "settings.import.stage.done"
}

const emit = defineEmits<{ imported: [report: ImportReport] }>()

const {t} = useI18n()

const source = ref<LauncherKind>("prism")
const path = ref("")
const launchers = ref<DetectedLauncher[]>([])

const current = computed(() => SOURCES.find(option => option.kind === source.value) ?? SOURCES[0]!)
const detected = computed(() => launchers.value.find(launcher => launcher.kind === source.value) ?? null)

const rows = computed(() => [
  {key: "libraries" as const, title: t("settings.import.rows.libraries"), hint: t(current.value.librariesHintKey)},
  {
    key: "assets" as const,
    title: t("settings.import.rows.assets"),
    hint: t("settings.import.rows.assets_hint")
  },
  {
    key: "java" as const,
    title: t("settings.import.rows.java"),
    hint: t("settings.import.rows.java_hint")
  },
  {key: "icons" as const, title: t("settings.import.rows.icons"), hint: t("settings.import.rows.icons_hint")},
  {
    key: "linkPacks" as const,
    title: t("settings.import.rows.link_packs"),
    hint: t("settings.import.rows.link_packs_hint")
  }
])

const scanning = ref(false)
const scanned = ref<ScannedInstance[] | null>(null)
const selected = ref<string[]>([])

const options = ref(defaultImportOptions())

const importStore = useImportStore()
const {progress, report, running} = storeToRefs(importStore)

const importable = computed(() => scanned.value?.filter(instance => !instance.blocked) ?? [])
const blocked = computed(() => scanned.value?.filter(instance => instance.blocked) ?? [])
const allSelected = computed(() => importable.value.length > 0 && selected.value.length === importable.value.length)

const canScan = computed(() => !!path.value.trim() && !scanning.value && !running.value)
const canImport = computed(() => selected.value.length > 0 && !running.value)

const isSelected = (folder: string) => selected.value.includes(folder)

function toggle(instance: ScannedInstance) {
  if (instance.blocked || running.value) return

  selected.value = isSelected(instance.folder)
      ? selected.value.filter(folder => folder !== instance.folder)
      : [...selected.value, instance.folder]
}

function toggleAll() {
  selected.value = allSelected.value ? [] : importable.value.map(instance => instance.folder)
}

async function detect() {
  launchers.value = await safeRun(() => call("detect_launchers")) ?? []

  if (detected.value && !path.value.trim()) {
    path.value = detected.value.path
  }
}

watch(source, () => {
  scanned.value = null
  selected.value = []
  report.value = null
  path.value = detected.value?.path ?? ""
})

async function browse() {
  const picked = await safeRun(() => call("pick_launcher_dir"))

  if (picked) {
    path.value = picked
    scanned.value = null
  }
}

async function scan() {
  if (!canScan.value) return

  scanning.value = true
  report.value = null

  try {
    const found = await call("scan_launcher_instances", {kind: source.value, path: path.value.trim()})

    scanned.value = found
    selected.value = found.filter(instance => !instance.blocked).map(instance => instance.folder)
  } catch (e) {
    scanned.value = null
    captureError(e, {context: {stage: t("settings.import.scan_stage", {launcher: current.value.label}), path: path.value}})
  } finally {
    scanning.value = false
  }
}

async function start() {
  if (!canImport.value) return

  importStore.begin()

  const result = await attempt(() => call("import_launcher_instances", {
    request: {
      kind: source.value,
      path: path.value.trim(),
      folders: selected.value,
      options: options.value
    }
  }), {context: {stage: t("settings.import.import_stage", {launcher: current.value.label}), path: path.value}})

  importStore.settle()

  if (result.ok) {
    emit("imported", result.value)
    await scan()
  }
}

const cancel = () => safeRun(() => call("cancel_import"))

onMounted(async () => {
  if (progress.value) source.value = progress.value.source

  await detect()
})
</script>

<template>
  <div class="space-y-7">
    <div class="grid gap-3 sm:grid-cols-2">
      <button
          v-for="option in SOURCES"
          :key="option.kind"
          type="button"
          class="group relative flex items-center gap-4 border border-line px-4 py-4 text-left transition-colors duration-300 hover:border-line-strong hover:bg-ink-700"
          :class="source === option.kind ? 'border-acid/60 bg-ink-700' : ''"
          :disabled="running"
          @click="source = option.kind"
      >
        <UIcon :name="option.icon" class="size-5 shrink-0" :class="source === option.kind ? 'text-acid' : 'text-fg-faint'"/>

        <div class="min-w-0 flex-1">
          <p class="truncate text-[13px] text-fg">{{ option.label }}</p>
          <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
            {{ $t(option.hintKey) }}
          </p>
        </div>
      </button>
    </div>

    <SettingsField
        :label="$t('settings.import.dir', { launcher: current.label })"
        :hint="detected
          ? $t('settings.import.detected', { count: detected.instances })
          : $t('settings.import.not_detected', { launcher: current.label })"
    >
      <div class="flex gap-2">
        <UInput
            v-model="path"
            :placeholder="current.placeholder"
            class="w-full"
            :disabled="running"
            :ui="{ base: 'font-mono text-[12px]' }"
        />

        <AppButton
            class="h-9 shrink-0 px-4 text-[10px] tracking-[0.18em]"
            icon="i-lucide-folder-open"
            :disabled="running"
            @click="browse"
        >
          {{ $t('common.browse') }}
        </AppButton>

        <AppButton
            class="h-9 shrink-0 px-4 text-[10px] tracking-[0.18em]"
            icon="i-lucide-search"
            :loading="scanning"
            :disabled="!canScan"
            @click="scan"
        >
          {{ scanning ? $t('settings.java.searching') : $t('settings.import.scan') }}
        </AppButton>
      </div>
    </SettingsField>

    <div v-if="scanned">
      <div class="mb-2 flex items-center justify-between gap-4">
        <span class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
          {{ $t('settings.import.selected', { selected: selected.length, total: importable.length }) }}
        </span>

        <AppButton
            tone="quiet"
            class="text-[10px] tracking-[0.18em]"
            :icon="allSelected ? 'i-lucide-square' : 'i-lucide-check-square'"
            :disabled="running || !importable.length"
            @click="toggleAll"
        >
          {{ allSelected ? $t('settings.import.unselect_all') : $t('settings.import.select_all') }}
        </AppButton>
      </div>

      <ul class="border-t border-line">
        <li
            v-for="instance in scanned"
            :key="instance.folder"
            class="group relative flex items-center gap-4 border-b border-line py-3.5 pl-4 pr-1 transition-colors duration-300"
            :class="instance.blocked ? 'opacity-45' : 'cursor-pointer hover:bg-ink-700'"
            @click="toggle(instance)"
        >
          <span
              class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
              :class="isSelected(instance.folder) ? 'scale-y-100' : 'scale-y-0'"
          />

          <UIcon
              :name="instance.blocked ? 'i-lucide-ban' : isSelected(instance.folder) ? 'i-lucide-check' : 'i-lucide-minus'"
              class="size-4 shrink-0"
              :class="isSelected(instance.folder) ? 'text-acid' : 'text-fg-faint'"
          />

          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] text-fg">{{ instance.name }}</p>
            <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ instance.minecraftVersion || '-' }} · {{ instance.loaderLabel }}
              <template v-if="instance.pack"> · {{ instance.pack.provider }}</template>
              <template v-if="formatPlaytime(instance.playtime?.totalSeconds ?? 0)">
                · {{ $t('settings.import.playtime', { playtime: formatPlaytime(instance.playtime.totalSeconds) }) }}
              </template>
            </p>
            <p v-if="instance.blocked" class="mt-1 truncate font-mono text-[10px] text-amber-400">
              {{ instance.blocked }}
            </p>
          </div>
        </li>
      </ul>

      <p
          v-if="!scanned.length"
          class="border-b border-line py-6 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
      >
        {{ $t('settings.import.empty') }}
      </p>

      <p v-else-if="blocked.length" class="mt-3 font-mono text-[10px] leading-relaxed text-fg-faint/70">
        {{ $t('settings.import.blocked', { count: blocked.length, launcher: current.label }) }}
      </p>
    </div>

    <div v-if="scanned?.length" class="space-y-4 border-t border-line pt-6">
      <div
          v-for="row in rows"
          :key="row.key"
          class="flex items-center justify-between gap-6"
      >
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ row.title }}</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">{{ row.hint }}</p>
        </div>

        <USwitch v-model="options[row.key]" size="lg" :disabled="running"/>
      </div>
    </div>

    <div v-if="running" class="border border-line bg-ink-700 px-5 py-4">
      <div class="flex items-center justify-between gap-4">
        <p class="min-w-0 truncate font-mono text-[10px] uppercase tracking-[0.24em] text-acid">
          {{ progress ? $t(STAGE_KEYS[progress.stage]) : $t('settings.import.preparing') }}
          <span v-if="progress?.step" class="text-fg-muted"> · {{ progress.step }}</span>
        </p>

        <AppButton
            tone="quiet"
            class="shrink-0 text-[10px] tracking-[0.18em]"
            icon="i-lucide-x"
            @click="cancel"
        >
          {{ $t('settings.import.cancel') }}
        </AppButton>
      </div>

      <p class="mt-3 font-mono text-[10px] text-fg-faint">
        <template v-if="progress">
          {{ $t('settings.import.files', { files: progress.stats.files, bytes: formatBytes(progress.stats.bytes) }) }}
          <template v-if="progress.stats.skipped"> · {{ $t('settings.import.skipped', { count: progress.stats.skipped }) }}</template>
          <template v-if="progress.total"> · {{ $t('settings.import.instances_progress', { done: progress.done, total: progress.total }) }}</template>
        </template>
        <template v-else>{{ $t('settings.import.reading', { launcher: current.label }) }}</template>
      </p>
    </div>

    <div v-else-if="report" class="border border-line bg-ink-700 px-5 py-4">
      <p class="font-mono text-[10px] uppercase tracking-[0.24em]" :class="report.cancelled ? 'text-amber-400' : 'text-acid'">
        {{ report.cancelled ? $t('settings.import.cancelled') : $t('settings.import.finished') }}
      </p>

      <p class="mt-3 text-[12px] leading-relaxed text-fg-muted">
        {{ $t('settings.import.report', {
          imported: report.imported.length,
          files: report.stats.files,
          bytes: formatBytes(report.stats.bytes)
        }) }}
      </p>

      <ul v-if="report.skipped.length" class="mt-3 space-y-1">
        <li
            v-for="skipped in report.skipped"
            :key="skipped.name"
            class="font-mono text-[10px] leading-relaxed text-amber-400"
        >
          {{ skipped.name }} - {{ skipped.reason }}
        </li>
      </ul>
    </div>

    <AppButton
        v-if="scanned?.length"
        block
        class="h-11 tracking-[0.2em]"
        icon="i-lucide-download"
        :loading="running"
        :disabled="!canImport"
        @click="start"
    >
      {{ running ? $t('settings.import.running') : $t('settings.import.start', { count: selected.length }) }}
    </AppButton>
  </div>
</template>
