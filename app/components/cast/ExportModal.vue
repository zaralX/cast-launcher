<script setup lang="ts">
import type { ExportResult, ExportScan, TreeEntry } from '~/types/cast'
import { EXPORT_STAGE_KEYS } from '~/types/cast'
import type { Instance } from '~/types/instance'
import { PACK_PROVIDER_LABELS } from '~/types/instance'

const props = defineProps<{ instance: Instance }>()

const open = defineModel<boolean>('open', { required: true })

type Step = 'files' | 'details' | 'exporting' | 'done'

const { t } = useI18n()
const castStore = useCastStore()
const accountStore = useAccountStore()

const FORM_ID = 'cast-export'

const step = ref<Step>('files')
const scanning = ref(false)
const scan = ref<ExportScan | null>(null)
const selected = ref(new Set<string>())
const result = ref<ExportResult | null>(null)
const useBase = ref(true)

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
  useBase.value = true
  load()
}, { immediate: true })

const base = computed(() => scan.value?.base ?? null)
const leaningOnBase = computed(() => !!base.value && useBase.value)

const baseLabel = computed(() => {
  const found = base.value
  if (!found) return ''

  return [found.name || PACK_PROVIDER_LABELS[found.provider], found.version].filter(Boolean).join(' ')
})

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
  let fromBase = 0
  const personal: string[] = []

  for (const entry of scan.value?.tree ?? []) {
    const items = entry.children?.length ? entry.children : [entry]
    const picked = items.filter(item => selected.value.has(item.key))

    if (picked.length && (entry.note === 'personal' || entry.note === 'world')) personal.push(entry.name)

    for (const item of picked) {
      if (leaningOnBase.value && item.inBase) {
        fromBase++
        continue
      }

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

  return { links, embedded, size, code, unchecked, fromBase, personal }
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
        useBase: leaningOnBase.value,
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
  <Dialog v-model:open="open">
    <DialogContent
      class="max-w-190"
      :dismissible="step !== 'exporting'"
      :show-close-button="step !== 'exporting'"
    >
      <DialogHeader>
        <DialogTitle>{{ $t('cast_export.heading') }}</DialogTitle>
        <DialogDescription class="truncate text-caption">
          {{ step === 'files' || step === 'details'
            ? `${instance.name} · ${$t('cast_export.step', { step: step === 'files' ? 1 : 2 })}`
            : instance.name }}
        </DialogDescription>
      </DialogHeader>

      <DialogBody>
        <div
          v-if="step === 'files'"
          class="space-y-4"
        >
          <p class="text-body text-fg-muted">
            {{ $t('cast_export.files_hint') }}
          </p>

          <div
            v-if="scanning && !scan"
            class="flex items-center gap-3 py-10"
          >
            <Spinner class="text-accent" />
            <span class="text-body text-fg-muted">{{ $t('cast_export.scanning') }}</span>
          </div>

          <template v-else-if="scan">
            <Alert
              v-if="scan.unchecked"
              variant="warning"
              icon="i-lucide-wifi-off"
            >
              {{ $t('cast_export.unchecked', { count: scan.unchecked }) }}
              <template #action>
                <Button
                  size="sm"
                  icon="i-lucide-rotate-cw"
                  :loading="scanning"
                  @click="load"
                >
                  {{ $t('cast_export.retry') }}
                </Button>
              </template>
            </Alert>

            <CastExportTree
              v-if="scan.tree.length"
              v-model:selected="selected"
              :tree="scan.tree"
            />

            <KitEmpty
              v-else
              icon="i-lucide-folder-open"
            >
              {{ $t('cast_export.empty') }}
            </KitEmpty>
          </template>

          <Button
            v-else
            icon="i-lucide-rotate-cw"
            @click="load"
          >
            {{ $t('cast_export.retry') }}
          </Button>
        </div>

        <form
          v-else-if="step === 'details'"
          :id="FORM_ID"
          class="space-y-5"
          @submit.prevent="run"
        >
          <div class="grid grid-cols-[minmax(0,1fr)_10rem] gap-4">
            <KitField
              :label="$t('cast_export.name')"
              for="cast-export-name"
            >
              <Input
                id="cast-export-name"
                v-model="form.name"
                size="lg"
              />
            </KitField>
            <KitField
              :label="$t('cast_export.version')"
              for="cast-export-version"
            >
              <Input
                id="cast-export-version"
                v-model="form.version"
                size="lg"
                font="mono"
              />
            </KitField>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <KitField
              :label="$t('cast_export.author')"
              for="cast-export-author"
            >
              <Input
                id="cast-export-author"
                v-model="form.author"
                :placeholder="$t('cast_export.optional')"
              />
            </KitField>
            <KitField
              :label="$t('cast_export.ram')"
              :hint="$t('cast_export.ram_hint')"
            >
              <KitNumberInput
                v-model="form.recommendedRam"
                :min="0"
                :unit="$t('common.unit.mb')"
                :placeholder="$t('cast_export.optional')"
              />
            </KitField>
          </div>

          <KitField
            :label="$t('cast_export.description')"
            for="cast-export-description"
          >
            <Textarea
              id="cast-export-description"
              v-model="form.description"
              :rows="2"
              autoresize
              :placeholder="$t('cast_export.optional')"
            />
          </KitField>

          <KitField
            :label="$t('cast_export.changelog')"
            :hint="$t('cast_export.changelog_hint')"
            for="cast-export-changelog"
          >
            <Textarea
              id="cast-export-changelog"
              v-model="form.changelog"
              :rows="2"
              autoresize
              :placeholder="$t('cast_export.optional')"
            />
          </KitField>

          <KitRows v-if="base">
            <KitRow
              :label="$t('cast_export.base.title', { name: baseLabel })"
              :description="useBase ? $t('cast_export.base.on_hint') : $t('cast_export.base.off_hint')"
            >
              <Switch v-model="useBase" />
            </KitRow>
          </KitRows>

          <KitStatGrid :columns="3">
            <KitStat
              :label="$t('cast_export.summary.links')"
              :value="String(summary.links)"
            />
            <KitStat
              :label="$t('cast_export.summary.embedded')"
              :value="String(summary.embedded)"
            />
            <KitStat
              :label="$t('cast_export.summary.size')"
              :value="formatBytes(summary.size)"
            />
          </KitStatGrid>

          <div class="space-y-2.5">
            <KitNote
              v-if="leaningOnBase"
              icon="i-lucide-layers"
              tone="accent"
            >
              {{ base?.compared
                ? $t('cast_export.base.from_base', { count: summary.fromBase })
                : $t('cast_export.base.not_compared') }}
            </KitNote>
            <KitNote
              v-if="summary.code"
              icon="i-lucide-package-open"
            >
              {{ $t('cast_export.code_hint', { count: summary.code }) }}
            </KitNote>
            <KitNote
              v-if="summary.personal.length"
              icon="i-lucide-user-round"
            >
              {{ $t('cast_export.personal_hint', { folders: summary.personal.join(', ') }) }}
            </KitNote>
            <KitNote
              :icon="scan?.defaults.knownPack ? 'i-lucide-refresh-cw' : 'i-lucide-sparkles'"
              tone="muted"
            >
              {{ scan?.defaults.knownPack ? $t('cast_export.known_pack') : $t('cast_export.new_pack') }}
            </KitNote>
          </div>
        </form>

        <div
          v-else-if="step === 'exporting'"
          class="space-y-3 py-6"
        >
          <div class="flex items-end justify-between gap-4">
            <p class="text-body text-fg-muted">
              {{ progress?.label ?? $t('cast_export.stage.collecting') }}
            </p>
            <span
              v-if="progress?.percent != null"
              class="text-heading font-semibold text-fg tabular-nums"
            >{{ progress.percent }}%</span>
          </div>
          <Progress :model-value="progress?.percent ?? null" />
          <p
            v-if="progress?.detail"
            class="font-mono text-caption text-fg-faint tabular-nums"
          >
            {{ progress.detail }}
          </p>
        </div>

        <div
          v-else-if="step === 'done' && result"
          class="space-y-4"
        >
          <KitNote
            icon="i-lucide-check"
            tone="success"
            class="text-fg"
          >
            {{ $t('cast_export.done', { size: formatBytes(result.size) }) }}
          </KitNote>

          <p
            class="truncate rounded-md bg-panel px-3.5 py-2.5 font-mono text-caption text-fg-muted ring-1 ring-card-line ring-inset"
            :title="result.path"
          >
            {{ result.path }}
          </p>

          <p class="text-caption text-fg-faint">
            {{ $t('cast_export.done_counts', { links: result.mods, embedded: result.embedded }) }}
            <template v-if="result.removed">
              · {{ $t('cast_export.done_removed', { count: result.removed }) }}
            </template>
          </p>

          <div v-if="result.skipped.length">
            <KitNote icon="i-lucide-ban">
              {{ $t('cast_export.skipped', { count: result.skipped.length }) }}
            </KitNote>
            <ul class="mt-1.5 space-y-0.5 pl-6.5 font-mono text-caption text-fg-faint">
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
      </DialogBody>

      <DialogFooter class="border-t border-line pt-3.5 pb-3.5">
        <template v-if="step === 'files'">
          <span
            v-if="scan"
            class="mr-auto truncate text-caption text-fg-faint"
          >
            {{ $t('cast_export.footer_summary', { links: summary.links, embedded: summary.embedded, size: formatBytes(summary.size) }) }}
          </span>
          <DialogClose as-child>
            <Button variant="ghost">
              {{ $t('common.cancel') }}
            </Button>
          </DialogClose>
          <Button
            variant="primary"
            :disabled="!canContinue || scanning"
            @click="step = 'details'"
          >
            {{ $t('cast_export.next') }}
          </Button>
        </template>

        <template v-else-if="step === 'details'">
          <Button
            variant="ghost"
            icon="i-lucide-arrow-left"
            class="mr-auto"
            @click="step = 'files'"
          >
            {{ $t('cast_export.back') }}
          </Button>
          <Button
            variant="primary"
            type="submit"
            :form="FORM_ID"
            icon="i-lucide-file-archive"
            :disabled="!canExport"
          >
            {{ $t('cast_export.export') }}
          </Button>
        </template>

        <Button
          v-else-if="step === 'exporting'"
          variant="ghost"
          icon="i-lucide-square"
          @click="cancel"
        >
          {{ $t('common.cancel') }}
        </Button>

        <template v-else-if="step === 'done' && result">
          <Button
            variant="ghost"
            icon="i-lucide-folder-open"
            class="mr-auto"
            @click="showInFolder(result.path)"
          >
            {{ $t('cast_export.show_in_folder') }}
          </Button>
          <Button
            variant="primary"
            @click="open = false"
          >
            {{ $t('cast_export.close') }}
          </Button>
        </template>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
