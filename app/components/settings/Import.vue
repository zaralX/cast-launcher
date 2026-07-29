<script setup lang="ts">
import {call, onLauncherEvent} from "~/types/backend"
import {
  defaultImportOptions,
  formatBytes,
  type DetectedLauncher,
  type ImportProgress,
  type ImportReport,
  type LauncherKind,
  type ScannedInstance
} from "~/types/import"

const SOURCES: { kind: LauncherKind, label: string, hint: string, icon: string, ready: boolean }[] = [
  {
    kind: "prism",
    label: "PrismLauncher",
    hint: "Сборки, ассеты, библиотеки и Java",
    icon: "i-lucide-package",
    ready: true
  },
  {
    kind: "modrinth",
    label: "Modrinth App",
    hint: "Пока не поддерживается",
    icon: "i-lucide-box",
    ready: false
  }
]

const STAGE_LABELS: Record<ImportProgress["stage"], string> = {
  shared: "Общие файлы",
  instances: "Сборки",
  done: "Готово"
}

const source = ref<LauncherKind>("prism")
const path = ref("")
const detected = ref<DetectedLauncher | null>(null)

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
  const launchers = await safeRun(() => call("detect_launchers")) ?? []

  detected.value = launchers.find(launcher => launcher.kind === "prism") ?? null

  if (detected.value && !path.value.trim()) {
    path.value = detected.value.path
  }
}

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
    const found = await call("scan_prism_instances", {path: path.value.trim()})

    scanned.value = found
    selected.value = found.filter(instance => !instance.blocked).map(instance => instance.folder)
  } catch (e) {
    scanned.value = null
    captureError(e, {context: {stage: "Сканирование PrismLauncher", path: path.value}})
  } finally {
    scanning.value = false
  }
}

async function start() {
  if (!canImport.value) return

  running.value = true
  progress.value = null
  report.value = null

  const result = await attempt(() => call("import_prism_instances", {
    request: {
      path: path.value.trim(),
      folders: selected.value,
      options: options.value
    }
  }), {context: {stage: "Перенос сборок из PrismLauncher", path: path.value}})

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
      title="Перенос из другого лаунчера"
      description="Копирует сборки вместе с мирами, модами, ассетами и библиотеками. Исходный лаунчер не меняется."
      icon="i-lucide-import"
  >
    <div class="space-y-7">
      <div class="grid gap-3 sm:grid-cols-2">
        <button
            v-for="option in SOURCES"
            :key="option.kind"
            type="button"
            class="group relative flex items-center gap-4 border border-line px-4 py-4 text-left transition-colors duration-300"
            :class="[
              option.ready ? 'hover:border-line-strong hover:bg-ink-700' : 'cursor-not-allowed opacity-40',
              source === option.kind && option.ready ? 'border-acid/60 bg-ink-700' : ''
            ]"
            :disabled="!option.ready || running"
            @click="source = option.kind"
        >
          <UIcon :name="option.icon" class="size-5 shrink-0" :class="source === option.kind ? 'text-acid' : 'text-fg-faint'"/>

          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] text-fg">{{ option.label }}</p>
            <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ option.hint }}
            </p>
          </div>

          <span
              v-if="!option.ready"
              class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint"
          >
            Скоро
          </span>
        </button>
      </div>

      <template v-if="source === 'prism'">
        <SettingsField
            label="Каталог данных PrismLauncher"
            :hint="detected ? `Найден автоматически: ${detected.instances} сборок` : 'PrismLauncher не найден — укажите папку вручную'"
        >
          <div class="flex gap-2">
            <UInput
                v-model="path"
                placeholder="%APPDATA%\PrismLauncher"
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
                  {{ instance.minecraftVersion || '—' }} · {{ instance.loaderLabel }}
                  <template v-if="instance.pack"> · {{ instance.pack.provider }}</template>
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
            {{ blocked.length }} сборок перенести нельзя — они останутся в PrismLauncher.
          </p>
        </div>

        <div v-if="scanned?.length" class="space-y-4 border-t border-line pt-6">
          <div
              v-for="row in [
                { key: 'libraries' as const, title: 'Библиотеки', hint: 'Библиотеки игры и загрузчиков, включая уже собранные Forge и NeoForge.' },
                { key: 'assets' as const, title: 'Ассеты', hint: 'Звуки и языки. Самая объёмная часть — зато качать заново не придётся.' },
                { key: 'java' as const, title: 'Java', hint: 'Рантаймы, которые Prism скачал у Mojang.' },
                { key: 'icons' as const, title: 'Иконки', hint: 'Иконки перенесённых сборок.' },
                { key: 'linkPacks' as const, title: 'Привязать модпаки', hint: 'Для паков с Modrinth останутся обновления версий.' }
              ]"
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
            <template v-else>Читаем каталог PrismLauncher</template>
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
              {{ skipped.name }} — {{ skipped.reason }}
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
      </template>
    </div>
  </SettingsPanel>
</template>
