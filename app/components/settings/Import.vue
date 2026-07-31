<script setup lang="ts">
import {call, onLauncherEvent} from "~/types/backend"
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
  hint: string
  icon: string
  placeholder: string
  librariesHint: string
}

const SOURCES: SourceInfo[] = [
  {
    kind: "prism",
    label: "PrismLauncher",
    hint: "Сборки, ассеты, библиотеки и Java",
    icon: "i-lucide-package",
    placeholder: "%APPDATA%\\PrismLauncher",
    librariesHint: "Библиотеки игры и загрузчиков, включая уже собранные Forge и NeoForge."
  },
  {
    kind: "modrinth",
    label: "Modrinth App",
    hint: "Сборки, ассеты, библиотеки и Java",
    icon: "i-lucide-box",
    placeholder: "%APPDATA%\\ModrinthApp",
    librariesHint: "Библиотеки игры и загрузчиков. Установщики Forge и NeoForge Modrinth App не хранит - их скачаем сами."
  }
]

const STAGE_LABELS: Record<ImportProgress["stage"], string> = {
  shared: "Общие файлы",
  instances: "Сборки",
  done: "Готово"
}

const source = ref<LauncherKind>("prism")
const path = ref("")
const launchers = ref<DetectedLauncher[]>([])

const current = computed(() => SOURCES.find(option => option.kind === source.value) ?? SOURCES[0]!)
const detected = computed(() => launchers.value.find(launcher => launcher.kind === source.value) ?? null)

const rows = computed(() => [
  {key: "libraries" as const, title: "Библиотеки", hint: current.value.librariesHint},
  {
    key: "assets" as const,
    title: "Ассеты",
    hint: "Рекомендовано при первом переносе чтобы не скачивать гигабайты снова."
  },
  {
    key: "java" as const,
    title: "Java",
    hint: "Рекомендовано при первом переносе. Скопируем уже скачанные java runtimes."
  },
  {key: "icons" as const, title: "Иконки", hint: "Иконки перенесённых сборок."},
  {
    key: "linkPacks" as const,
    title: "Привязать модпаки",
    hint: "Для паков с Modrinth и CurseForge останется связь для обновления версий."
  }
])

const scanning = ref(false)
const scanned = ref<ScannedInstance[] | null>(null)
const selected = ref<string[]>([])

const options = ref(defaultImportOptions())

const running = ref(false)
const progress = ref<ImportProgress | null>(null)
const report = ref<ImportReport | null>(null)

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
    captureError(e, {context: {stage: `Сканирование ${current.value.label}`, path: path.value}})
  } finally {
    scanning.value = false
  }
}

async function start() {
  if (!canImport.value) return

  running.value = true
  progress.value = null
  report.value = null

  const result = await attempt(() => call("import_launcher_instances", {
    request: {
      kind: source.value,
      path: path.value.trim(),
      folders: selected.value,
      options: options.value
    }
  }), {context: {stage: `Перенос сборок из ${current.value.label}`, path: path.value}})

  running.value = false
  progress.value = null

  if (result.ok) {
    report.value = result.value
    await scan()
  }
}

const cancel = () => safeRun(() => call("cancel_import"))

let unlisten: (() => void) | null = null

onMounted(async () => {
  await detect()

  unlisten = await onLauncherEvent(event => {
    if (event.type !== "import") return

    progress.value = event.stage === "done" ? null : event
  })
})

onBeforeUnmount(() => unlisten?.())
</script>

<template>
  <SettingsPanel
      index="04"
      title="Копирование из другого лаунчера"
      icon="i-lucide-import"
  >
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
              {{ option.hint }}
            </p>
          </div>
        </button>
      </div>

      <SettingsField
          :label="`Каталог данных ${current.label}`"
          :hint="detected ? `Найдено автоматически: ${detected.instances} сборок` : `${current.label} не найден - укажите папку вручную`"
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
            Обзор
          </AppButton>

          <AppButton
              class="h-9 shrink-0 px-4 text-[10px] tracking-[0.18em]"
              icon="i-lucide-search"
              :loading="scanning"
              :disabled="!canScan"
              @click="scan"
          >
            {{ scanning ? 'Поиск' : 'Сканировать' }}
          </AppButton>
        </div>
      </SettingsField>

      <div v-if="scanned">
        <div class="mb-2 flex items-center justify-between gap-4">
          <span class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            Сборки · выбрано {{ selected.length }} из {{ importable.length }}
          </span>

          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.18em]"
              :icon="allSelected ? 'i-lucide-square' : 'i-lucide-check-square'"
              :disabled="running || !importable.length"
              @click="toggleAll"
          >
            {{ allSelected ? 'Снять всё' : 'Выбрать всё' }}
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
                  · наиграно {{ formatPlaytime(instance.playtime.totalSeconds) }}
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
          Сборок в этом каталоге нет
        </p>

        <p v-else-if="blocked.length" class="mt-3 font-mono text-[10px] leading-relaxed text-fg-faint/70">
          {{ blocked.length }} сборок перенести нельзя - они останутся в {{ current.label }}.
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
            {{ progress ? STAGE_LABELS[progress.stage] : 'Подготовка' }}
            <span v-if="progress?.step" class="text-fg-muted"> · {{ progress.step }}</span>
          </p>

          <AppButton
              tone="quiet"
              class="shrink-0 text-[10px] tracking-[0.18em]"
              icon="i-lucide-x"
              @click="cancel"
          >
            Отменить
          </AppButton>
        </div>

        <p class="mt-3 font-mono text-[10px] text-fg-faint">
          <template v-if="progress">
            {{ progress.stats.files }} файлов · {{ formatBytes(progress.stats.bytes) }}
            <template v-if="progress.stats.skipped"> · пропущено {{ progress.stats.skipped }}</template>
            <template v-if="progress.total"> · сборок {{ progress.done }}/{{ progress.total }}</template>
          </template>
          <template v-else>Читаем каталог {{ current.label }}</template>
        </p>
      </div>

      <div v-else-if="report" class="border border-line bg-ink-700 px-5 py-4">
        <p class="font-mono text-[10px] uppercase tracking-[0.24em]" :class="report.cancelled ? 'text-amber-400' : 'text-acid'">
          {{ report.cancelled ? 'Перенос прерван' : 'Перенос завершён' }}
        </p>

        <p class="mt-3 text-[12px] leading-relaxed text-fg-muted">
          Перенесено сборок: {{ report.imported.length }}.
          Скопировано {{ report.stats.files }} файлов ({{ formatBytes(report.stats.bytes) }}).
          Сборки появятся в списке и доустановятся при первом запуске установки.
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
        {{ running ? 'Переносим' : `Перенести (${selected.length})` }}
      </AppButton>
    </div>
  </SettingsPanel>
</template>
