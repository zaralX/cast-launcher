<script setup lang="ts">
import type { ExportResult, ExportScan, TreeEntry } from '~/types/cast'
import { EXPORT_STAGE_KEYS } from '~/types/cast'
import type { Instance } from '~/types/instance'

const props = defineProps<{ instance: Instance }>()

const open = defineModel<boolean>('open', { required: true })

type Step = 'files' | 'details' | 'exporting' | 'done'

const { t } = useI18n()
const castStore = useCastStore()
const accountStore = useAccountStore()

const step = ref<Step>('files')
const scanning = ref(false)
const scan = ref<ExportScan | null>(null)
const selected = ref(new Set<string>())
const result = ref<ExportResult | null>(null)

const form = ref({
  name: '',
  version: '',
  author: '',
  description: '',
  changelog: '',
  recommendedRam: '' as string | number,
})

const CODE_FOLDER = 'mods/'

function initialSelection(tree: TreeEntry[]) {
  const picked = new Set<string>()

  for (const entry of tree) {
    const items = entry.children?.length ? entry.children : [entry]

    for (const item of items) {
      if (item.selected && item.note !== 'forbidden') picked.add(item.key)
    }
  }

  return picked
}

async function load() {
  scanning.value = true

  const scanned = await attempt(() => call('cast_export_scan', { instanceId: props.instance.id }), {
    context: { instanceId: props.instance.id, action: t('cast_export.scan_action') },
  })

  scanning.value = false

  if (!scanned.ok) return

  const firstScan = !scan.value
  scan.value = scanned.value

  if (!firstScan) return

  const defaults = scanned.value.defaults

  selected.value = initialSelection(scanned.value.tree)
  form.value = {
    name: defaults.name,
    version: defaults.version,
    author: defaults.author || accountStore.selected?.name || '',
    description: defaults.description,
    changelog: '',
    recommendedRam: defaults.recommendedRam ?? '',
  }
}

watch(open, (opened) => {
  if (!opened) return

  step.value = 'files'
  scan.value = null
  result.value = null
  load()
}, { immediate: true })

// A fully picked folder goes as a whole, so files that appear in it later travel too.
const include = computed(() => {
  const keys: string[] = []

  for (const entry of scan.value?.tree ?? []) {
    const children = (entry.children ?? []).filter(child => child.note !== 'forbidden')

    if (!entry.children?.length) {
      if (selected.value.has(entry.key)) keys.push(entry.key)
      continue
    }

    const picked = children.filter(child => selected.value.has(child.key))

    if (picked.length && picked.length === children.length) keys.push(entry.key)
    else keys.push(...picked.map(child => child.key))
  }

  return keys
})

const summary = computed(() => {
  let links = 0
  let embedded = 0
  let size = 0
  let code = 0
  let unchecked = 0
  const personal: string[] = []

  for (const entry of scan.value?.tree ?? []) {
    const items = entry.children?.length ? entry.children : [entry]
    const picked = items.filter(item => selected.value.has(item.key))

    if (picked.length && (entry.note === 'personal' || entry.note === 'world')) personal.push(entry.name)

    for (const item of picked) {
      const kind = item.source?.kind

      if (kind === 'modrinth' || kind === 'curseforge') {
        links++
        continue
      }

      embedded += item.dir ? item.files : 1
      size += item.size

      if (kind === 'unchecked') unchecked++
      if (kind && !item.dir && item.key.startsWith(CODE_FOLDER)) code++
    }
  }

  return { links, embedded, size, code, unchecked, personal }
})

const canContinue = computed(() => include.value.length > 0)

const canExport = computed(() =>
  canContinue.value && !!form.value.name.trim() && !!form.value.version.trim())

async function run() {
  if (!canExport.value) return

  step.value = 'exporting'
  castStore.finishExport()

  const ram = Number(form.value.recommendedRam)

  try {
    const exported = await call('cast_export', {
      instanceId: props.instance.id,
      request: {
        name: form.value.name.trim(),
        version: form.value.version.trim(),
        author: form.value.author.trim(),
        description: form.value.description.trim(),
        changelog: form.value.changelog.trim(),
        recommendedRam: Number.isFinite(ram) && ram > 0 ? Math.round(ram) : undefined,
        include: include.value,
      },
      dialog: { title: t('dialog.cast_export.title'), filter: t('dialog.cast_export.filter') },
    })

    if (!exported) {
      step.value = 'details'
      return
    }

    result.value = exported
    step.value = 'done'
  }
  catch (error) {
    step.value = 'details'

    // A cancelled export is what the player asked for, not a failure.
    if (error instanceof LauncherError && error.code === 'INSTALL_ABORTED') return

    captureError(error, { context: { instanceId: props.instance.id, action: t('cast_export.action') } })
  }
  finally {
    castStore.finishExport()
  }
}

const cancel = () => safeRun(() => call('cancel_cast_export'))

const progress = computed(() => {
  const current = castStore.exporting
  if (!current || current.instanceId !== props.instance.id) return null

  const percent = current.stage === 'packing' && current.total > 0
    ? Math.min(100, Math.round(current.done / current.total * 100))
    : null

  return {
    label: t(EXPORT_STAGE_KEYS[current.stage]),
    percent,
    detail: current.stage === 'packing' ? `${formatBytes(current.done)} / ${formatBytes(current.total)}` : '',
  }
})

function folderOf(path: string) {
  return path.replace(/[\\/][^\\/]*$/, '')
}

const showInFolder = (path: string) => safeRun(() => call('open_path', { path: folderOf(path) }))
</script>

<template>
  <UModal
    v-model:open="open"
    :title="$t('cast_export.title', { name: instance.name })"
    :dismissible="step !== 'exporting'"
    :close="step !== 'exporting'"
    :ui="{ content: 'max-w-3xl' }"
  >
    <template #body>
      <div
        v-if="step === 'files'"
        class="space-y-5"
      >
        <p class="text-[12px] leading-relaxed text-fg-muted">
          {{ $t('cast_export.files_hint') }}
        </p>

        <div
          v-if="scanning && !scan"
          class="flex items-center gap-3 py-10"
        >
          <UIcon
            name="i-lucide-loader"
            class="size-4 animate-spin text-acid"
          />
          <span class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ $t('cast_export.scanning') }}</span>
        </div>

        <template v-else-if="scan">
          <p
            v-if="scan.unchecked"
            class="flex items-start justify-between gap-4 border border-amber-400/30 bg-ink-900 px-4 py-3 text-[12px] leading-relaxed text-fg-muted"
          >
            <span class="flex items-start gap-2.5">
              <UIcon
                name="i-lucide-wifi-off"
                class="mt-0.5 size-3.5 shrink-0 text-amber-400"
              />
              {{ $t('cast_export.unchecked', { count: scan.unchecked }) }}
            </span>
            <AppButton
              tone="quiet"
              class="shrink-0 text-[10px] tracking-[0.18em]"
              icon="i-lucide-rotate-cw"
              :loading="scanning"
              @click="load"
            >
              {{ $t('cast_export.retry') }}
            </AppButton>
          </p>

          <CastExportTree
            v-model:selected="selected"
            :tree="scan.tree"
          />

          <p
            v-if="!scan.tree.length"
            class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            {{ $t('cast_export.empty') }}
          </p>
        </template>
      </div>

      <form
        v-else-if="step === 'details'"
        class="space-y-6"
        @submit.prevent="run"
      >
        <div class="grid gap-5 sm:grid-cols-[minmax(0,1fr)_10rem]">
          <SettingsField :label="$t('cast_export.name')">
            <UInput
              v-model="form.name"
              size="lg"
              class="w-full"
              :ui="{ base: 'font-unbounded text-[15px] tracking-[-0.03em]' }"
            />
          </SettingsField>

          <SettingsField :label="$t('cast_export.version')">
            <UInput
              v-model="form.version"
              size="lg"
              class="w-full"
              :ui="{ base: 'font-mono' }"
            />
          </SettingsField>
        </div>

        <div class="grid gap-5 sm:grid-cols-2">
          <SettingsField :label="$t('cast_export.author')">
            <UInput
              v-model="form.author"
              class="w-full"
              :placeholder="$t('cast_export.optional')"
            />
          </SettingsField>

          <SettingsField
            :label="$t('cast_export.ram')"
            :hint="$t('cast_export.ram_hint')"
          >
            <UInput
              v-model="form.recommendedRam"
              type="number"
              :min="0"
              class="w-full"
              :placeholder="$t('cast_export.optional')"
              :ui="{ base: 'font-mono tabular-nums' }"
            >
              <template #trailing>
                <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">MB</span>
              </template>
            </UInput>
          </SettingsField>
        </div>

        <SettingsField :label="$t('cast_export.description')">
          <UTextarea
            v-model="form.description"
            :rows="2"
            autoresize
            class="w-full"
            :placeholder="$t('cast_export.optional')"
          />
        </SettingsField>

        <SettingsField
          :label="$t('cast_export.changelog')"
          :hint="$t('cast_export.changelog_hint')"
        >
          <UTextarea
            v-model="form.changelog"
            :rows="3"
            autoresize
            class="w-full"
            :placeholder="$t('cast_export.optional')"
          />
        </SettingsField>

        <dl class="grid grid-cols-3 border border-line">
          <div class="px-4 py-3">
            <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ $t('cast_export.summary.links') }}
            </dt>
            <dd class="mt-1.5 font-unbounded text-[13px] tracking-[-0.03em] text-fg">
              {{ summary.links }}
            </dd>
          </div>
          <div class="border-l border-line px-4 py-3">
            <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ $t('cast_export.summary.embedded') }}
            </dt>
            <dd class="mt-1.5 font-unbounded text-[13px] tracking-[-0.03em] text-fg">
              {{ summary.embedded }}
            </dd>
          </div>
          <div class="border-l border-line px-4 py-3">
            <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ $t('cast_export.summary.size') }}
            </dt>
            <dd class="mt-1.5 font-unbounded text-[13px] tracking-[-0.03em] text-fg">
              {{ formatBytes(summary.size) }}
            </dd>
          </div>
        </dl>

        <div class="space-y-2.5 text-[12px] leading-relaxed text-fg-muted">
          <p
            v-if="summary.code"
            class="flex items-start gap-2.5"
          >
            <UIcon
              name="i-lucide-package-open"
              class="mt-0.5 size-3.5 shrink-0 text-amber-400"
            />
            {{ $t('cast_export.code_hint', { count: summary.code }) }}
          </p>

          <p
            v-if="summary.personal.length"
            class="flex items-start gap-2.5"
          >
            <UIcon
              name="i-lucide-user-round"
              class="mt-0.5 size-3.5 shrink-0 text-amber-400"
            />
            {{ $t('cast_export.personal_hint', { folders: summary.personal.join(', ') }) }}
          </p>

          <p class="flex items-start gap-2.5">
            <UIcon
              :name="scan?.defaults.knownPack ? 'i-lucide-refresh-cw' : 'i-lucide-sparkles'"
              class="mt-0.5 size-3.5 shrink-0 text-fg-faint"
            />
            {{ scan?.defaults.knownPack ? $t('cast_export.known_pack') : $t('cast_export.new_pack') }}
          </p>
        </div>
      </form>

      <div
        v-else-if="step === 'exporting'"
        class="space-y-4 py-6"
      >
        <header class="flex items-end justify-between gap-4">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-muted">
            {{ progress?.label ?? $t('cast_export.stage.collecting') }}
          </p>
          <span
            v-if="progress?.percent !== null && progress?.percent !== undefined"
            class="font-unbounded text-[22px] font-semibold leading-none tracking-[-0.05em] text-fg"
          >
            {{ progress.percent }}<span class="text-[12px] text-acid">%</span>
          </span>
        </header>

        <div class="h-px w-full overflow-hidden bg-line">
          <div
            v-if="progress?.percent !== null && progress?.percent !== undefined"
            class="h-px bg-acid transition-[width] duration-500 ease-deck"
            :style="{ width: `${progress.percent}%` }"
          />
          <div
            v-else
            class="h-px w-1/3 animate-pulse bg-acid"
          />
        </div>

        <p
          v-if="progress?.detail"
          class="font-mono text-[10px] tabular-nums text-fg-faint"
        >
          {{ progress.detail }}
        </p>
      </div>

      <div
        v-else-if="step === 'done' && result"
        class="space-y-5"
      >
        <p class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg">
          <UIcon
            name="i-lucide-check"
            class="mt-0.5 size-3.5 shrink-0 text-acid"
          />
          {{ $t('cast_export.done', { size: formatBytes(result.size) }) }}
        </p>

        <p
          class="truncate border border-line px-4 py-3 font-mono text-[11px] text-fg-muted"
          :title="result.path"
        >
          {{ result.path }}
        </p>

        <p class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">
          {{ $t('cast_export.done_counts', { links: result.mods, embedded: result.embedded }) }}
        </p>

        <div
          v-if="result.skipped.length"
          class="text-[12px] leading-relaxed text-fg-muted"
        >
          <p class="flex items-start gap-2.5">
            <UIcon
              name="i-lucide-ban"
              class="mt-0.5 size-3.5 shrink-0 text-amber-400"
            />
            {{ $t('cast_export.skipped', { count: result.skipped.length }) }}
          </p>
          <ul class="mt-1.5 space-y-0.5 pl-6 font-mono text-[10px] text-fg-faint">
            <li
              v-for="file in result.skipped.slice(0, 6)"
              :key="file"
              class="truncate"
            >
              {{ file }}
            </li>
          </ul>
        </div>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-between gap-3">
        <AppButton
          v-if="step === 'details'"
          tone="quiet"
          class="text-[10px] tracking-[0.18em]"
          icon="i-lucide-arrow-left"
          @click="step = 'files'"
        >
          {{ $t('cast_export.back') }}
        </AppButton>
        <span v-else />

        <AppButton
          v-if="step === 'files'"
          class="h-9 text-[10px] tracking-[0.18em]"
          icon="i-lucide-arrow-right"
          :disabled="!canContinue || scanning"
          @click="step = 'details'"
        >
          {{ $t('cast_export.next') }}
        </AppButton>

        <AppButton
          v-else-if="step === 'details'"
          class="h-9 text-[10px] tracking-[0.18em]"
          icon="i-lucide-file-down"
          :disabled="!canExport"
          @click="run"
        >
          {{ $t('cast_export.export') }}
        </AppButton>

        <AppButton
          v-else-if="step === 'exporting'"
          tone="quiet"
          class="text-[10px] tracking-[0.18em]"
          icon="i-lucide-square"
          @click="cancel"
        >
          {{ $t('common.cancel') }}
        </AppButton>

        <div
          v-else-if="step === 'done' && result"
          class="flex gap-2"
        >
          <AppButton
            tone="quiet"
            class="text-[10px] tracking-[0.18em]"
            icon="i-lucide-folder-open"
            @click="showInFolder(result.path)"
          >
            {{ $t('cast_export.show_in_folder') }}
          </AppButton>

          <AppButton
            class="h-9 text-[10px] tracking-[0.18em]"
            @click="open = false"
          >
            {{ $t('cast_export.close') }}
          </AppButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
