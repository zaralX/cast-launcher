<script setup lang="ts">
import type { DetectedLauncher, ImportProgress, ImportReport, LauncherKind, ScannedInstance } from '~/types/import'

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
    kind: 'prism',
    label: 'PrismLauncher',
    hintKey: 'settings.import.prism.hint',
    icon: 'i-lucide-package',
    placeholder: '%APPDATA%\\PrismLauncher',
    librariesHintKey: 'settings.import.prism.libraries_hint',
  },
  {
    kind: 'modrinth',
    label: 'Modrinth App',
    hintKey: 'settings.import.modrinth.hint',
    icon: 'i-lucide-box',
    placeholder: '%APPDATA%\\ModrinthApp',
    librariesHintKey: 'settings.import.modrinth.libraries_hint',
  },
]

const STAGE_KEYS: Record<ImportProgress['stage'], string> = {
  shared: 'settings.import.stage.shared',
  instances: 'settings.import.stage.instances',
  done: 'settings.import.stage.done',
}

const emit = defineEmits<{ imported: [report: ImportReport] }>()

const { t } = useI18n()

const source = ref<LauncherKind>('prism')
const path = ref('')
const launchers = ref<DetectedLauncher[]>([])

const current = computed(() => SOURCES.find(option => option.kind === source.value) ?? SOURCES[0]!)
const detected = computed(() => launchers.value.find(launcher => launcher.kind === source.value) ?? null)

const rows = computed(() => [
  { key: 'libraries' as const, title: t('settings.import.rows.libraries'), hint: t(current.value.librariesHintKey) },
  {
    key: 'assets' as const,
    title: t('settings.import.rows.assets'),
    hint: t('settings.import.rows.assets_hint'),
  },
  {
    key: 'java' as const,
    title: t('settings.import.rows.java'),
    hint: t('settings.import.rows.java_hint'),
  },
  {
    key: 'icons' as const,
    title: t('settings.import.rows.icons'),
    hint: undefined,
  },
  {
    key: 'linkPacks' as const,
    title: t('settings.import.rows.link_packs'),
    hint: t('settings.import.rows.link_packs_hint'),
  },
])

const scanning = ref(false)
const scanned = ref<ScannedInstance[] | null>(null)
const selected = ref<string[]>([])

const options = ref(defaultImportOptions())

const importStore = useImportStore()
const { progress, report, running } = storeToRefs(importStore)

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
  launchers.value = await safeRun(() => call('detect_launchers')) ?? []

  if (detected.value && !path.value.trim()) {
    path.value = detected.value.path
  }
}

watch(source, () => {
  scanned.value = null
  selected.value = []
  report.value = null
  path.value = detected.value?.path ?? ''
})

async function browse() {
  const picked = await safeRun(() => call('pick_launcher_dir', { dialog: { title: t('dialog.launcher_dir.title') } }))

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
    const found = await call('scan_launcher_instances', { kind: source.value, path: path.value.trim() })

    scanned.value = found
    selected.value = found.filter(instance => !instance.blocked).map(instance => instance.folder)
  }
  catch (e) {
    scanned.value = null
    captureError(e, { context: { stage: t('settings.import.scan_stage', { launcher: current.value.label }), path: path.value } })
  }
  finally {
    scanning.value = false
  }
}

async function start() {
  if (!canImport.value) return

  importStore.begin()

  const result = await attempt(() => call('import_launcher_instances', {
    request: {
      kind: source.value,
      path: path.value.trim(),
      folders: selected.value,
      options: options.value,
    },
  }), { context: { stage: t('settings.import.import_stage', { launcher: current.value.label }), path: path.value } })

  importStore.settle()

  if (result.ok) {
    emit('imported', result.value)
    await scan()
  }
}

const cancel = () => safeRun(() => call('cancel_import'))

onMounted(async () => {
  if (progress.value) source.value = progress.value.source

  await detect()
})
</script>

<template>
  <div class="space-y-7">
    <RadioGroup
      v-model="source"
      variant="cards"
      :disabled="running"
      class="grid-cols-2"
    >
      <RadioGroupItem
        v-for="option in SOURCES"
        :key="option.kind"
        :value="option.kind"
      >
        <Icon
          :name="option.icon"
          class="size-5 shrink-0 text-fg-faint group-data-[state=checked]:text-accent"
        />

        <div class="min-w-0 flex-1">
          <p class="truncate font-medium text-fg">
            {{ option.label }}
          </p>
          <p class="mt-0.5 truncate text-caption text-fg-faint">
            {{ $t(option.hintKey) }}
          </p>
        </div>
      </RadioGroupItem>
    </RadioGroup>

    <KitField
      :label="$t('settings.import.dir', { launcher: current.label })"
      :hint="detected
        ? $t('settings.import.detected', { count: detected.instances })
        : $t('settings.import.not_detected', { launcher: current.label })"
      for="import-launcher-path"
    >
      <div class="flex gap-2">
        <Input
          id="import-launcher-path"
          v-model="path"
          :placeholder="current.placeholder"
          size="lg"
          font="mono"
          :disabled="running"
        />

        <Button
          icon="i-lucide-folder-open"
          :disabled="running"
          @click="browse"
        >
          {{ $t('common.browse') }}
        </Button>

        <Button
          icon="i-lucide-search"
          :loading="scanning"
          :disabled="!canScan"
          @click="scan"
        >
          {{ scanning ? $t('settings.java.searching') : $t('settings.import.scan') }}
        </Button>
      </div>
    </KitField>

    <div v-if="scanned">
      <div class="mb-2 flex items-center justify-between gap-4">
        <Label>
          {{ $t('settings.import.selected', { selected: selected.length, total: importable.length }) }}
        </Label>

        <Button
          variant="ghost"
          size="sm"
          class="-mr-2.5"
          :disabled="running || !importable.length"
          @click="toggleAll"
        >
          {{ allSelected ? $t('settings.import.unselect_all') : $t('settings.import.select_all') }}
        </Button>
      </div>

      <ul
        v-if="scanned.length"
        class="max-h-80 divide-y divide-line overflow-y-auto rounded-md bg-panel ring-1 ring-card-line ring-inset"
      >
        <KitListItem
          v-for="instance in scanned"
          :key="instance.folder"
          :disabled="!!instance.blocked"
          class="rounded-none px-4"
          @select="toggle(instance)"
        >
          <template #leading>
            <Checkbox
              :model-value="isSelected(instance.folder)"
              :disabled="!!instance.blocked"
              tabindex="-1"
              aria-hidden="true"
              class="pointer-events-none"
            />
          </template>

          <p class="truncate font-medium text-fg">
            {{ instance.name }}
          </p>
          <p class="truncate text-caption text-fg-faint">
            {{ instance.minecraftVersion || '-' }} · {{ instance.loaderLabel }}
            <template v-if="instance.pack">
              · {{ instance.pack.provider }}
            </template>
            <template v-if="formatPlaytime(instance.playtime?.totalSeconds ?? 0)">
              · {{ $t('settings.import.playtime', { playtime: formatPlaytime(instance.playtime.totalSeconds) }) }}
            </template>
          </p>
          <p
            v-if="instance.blocked"
            class="truncate text-caption text-warning"
          >
            {{ uiText(instance.blocked) }}
          </p>
        </KitListItem>
      </ul>

      <KitEmpty
        v-else
        icon="i-lucide-folder-search"
      >
        {{ $t('settings.import.empty') }}
      </KitEmpty>

      <p
        v-if="blocked.length"
        class="mt-3 text-caption text-fg-faint"
      >
        {{ $t('settings.import.blocked', { count: blocked.length, launcher: current.label }) }}
      </p>
    </div>

    <KitRows v-if="scanned?.length">
      <KitRow
        v-for="row in rows"
        :key="row.key"
        :label="row.title"
        :description="row.hint"
      >
        <Switch
          v-model="options[row.key]"
          :disabled="running"
        />
      </KitRow>
    </KitRows>

    <Alert
      v-if="running"
      variant="accent"
      icon="i-lucide-loader"
    >
      <p class="truncate font-medium text-fg">
        {{ progress ? $t(STAGE_KEYS[progress.stage]) : $t('settings.import.preparing') }}
        <span
          v-if="progress?.step"
          class="font-normal text-fg-muted"
        > · {{ uiText(progress.step) }}</span>
      </p>

      <p class="mt-0.5 text-caption text-fg-faint">
        <template v-if="progress">
          {{ $t('settings.import.files', { files: progress.stats.files, bytes: formatBytes(progress.stats.bytes) }) }}
          <template v-if="progress.stats.skipped">
            · {{ $t('settings.import.skipped', { count: progress.stats.skipped }) }}
          </template>
          <template v-if="progress.total">
            · {{ $t('settings.import.instances_progress', { done: progress.done, total: progress.total }) }}
          </template>
        </template>
        <template v-else>
          {{ $t('settings.import.reading', { launcher: current.label }) }}
        </template>
      </p>

      <template #action>
        <Button
          variant="ghost"
          size="sm"
          icon="i-lucide-x"
          @click="cancel"
        >
          {{ $t('settings.import.cancel') }}
        </Button>
      </template>
    </Alert>

    <Alert
      v-else-if="report"
      :variant="report.cancelled ? 'warning' : 'accent'"
      :icon="report.cancelled ? 'i-lucide-circle-slash' : 'i-lucide-circle-check'"
    >
      <p class="font-medium text-fg">
        {{ report.cancelled ? $t('settings.import.cancelled') : $t('settings.import.finished') }}
      </p>

      <p class="mt-0.5">
        {{ $t('settings.import.report', {
          imported: report.imported.length,
          files: report.stats.files,
          bytes: formatBytes(report.stats.bytes),
        }) }}
      </p>

      <ul
        v-if="report.skipped.length"
        class="mt-2 space-y-1"
      >
        <li
          v-for="skipped in report.skipped"
          :key="skipped.name"
          class="text-caption text-warning"
        >
          {{ skipped.name }} - {{ uiText(skipped.reason) }}
        </li>
      </ul>
    </Alert>

    <Button
      v-if="scanned?.length"
      variant="primary"
      size="lg"
      icon="i-lucide-download"
      class="w-full"
      :loading="running"
      :disabled="!canImport"
      @click="start"
    >
      {{ running ? $t('settings.import.running') : $t('settings.import.start', { count: selected.length }) }}
    </Button>
  </div>
</template>
