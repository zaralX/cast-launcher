<script setup lang="ts">
import type { InstanceLogFile } from '~/types/instance'

const props = defineProps<{ instanceId: string }>()

const LIVE = 'live'

const { t, locale } = useI18n()
const instanceStore = useInstanceStore()
const { logs } = storeToRefs(instanceStore)
const toast = useAppToast()

const files = ref<InstanceLogFile[]>([])
const source = ref<string>(LIVE)
const fileText = ref('')
const loading = ref(false)
const filter = ref('')
const autoscroll = ref(true)
const wrap = ref(true)

const viewer = ref<HTMLElement | null>(null)

const running = computed(() => instanceStore.isRunning(props.instanceId))
const live = computed(() => logs.value[props.instanceId] ?? [])
const isLive = computed(() => source.value === LIVE)

const sources = computed(() => [
  { label: running.value ? t('instance.logs.live_running') : t('instance.logs.live'), value: LIVE },
  ...files.value.map(file => ({ label: `${fileLabel(file)} · ${size(file.size)}`, value: file.name })),
])

interface Line {
  text: string
  tone: '' | 'error' | 'warn'
}

const all = computed<Line[]>(() => {
  if (isLive.value) {
    return live.value.map(line => ({ text: line.line, tone: line.isError ? 'error' : tone(line.line) }))
  }

  const text = fileText.value.replace(/\n$/, '')
  return text ? text.split(/\r?\n/).map(line => ({ text: line, tone: tone(line) })) : []
})

const lines = computed<Line[]>(() => {
  const needle = filter.value.trim().toLowerCase()
  if (!needle) return all.value

  return all.value.filter(line => line.text.toLowerCase().includes(needle))
})

function tone(line: string): Line['tone'] {
  if (/\b(ERROR|FATAL|SEVERE)\b/.test(line) || /Exception|\bat [\w.$]+\(/.test(line)) return 'error'
  if (/\bWARN(ING)?\b/.test(line)) return 'warn'
  return ''
}

function fileLabel(file: InstanceLogFile) {
  const stamp = Number(file.name.replace(/\.log$/, ''))
  const date = new Date(Number.isFinite(stamp) && stamp > 0 ? stamp : file.modified)

  return Number.isNaN(date.getTime()) ? file.name : date.toLocaleString(locale.value)
}

function size(bytes: number) {
  if (bytes < 1024) return `${bytes} ${t('common.unit.b')}`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} ${t('common.unit.kb')}`
  return `${(bytes / 1024 / 1024).toFixed(1)} ${t('common.unit.mb')}`
}

async function loadFiles() {
  const result = await attempt(
    () => call('list_instance_logs', { instanceId: props.instanceId }),
    { context: { instanceId: props.instanceId, action: t('instance.logs.list_action') } },
  )

  if (!result.ok) return

  files.value = result.value

  if (!isLive.value && !files.value.some(file => file.name === source.value)) {
    source.value = LIVE
  }
}

async function loadFile(name: string) {
  loading.value = true

  const result = await attempt(
    () => call('read_instance_log', { instanceId: props.instanceId, name }),
    { context: { instanceId: props.instanceId, action: t('instance.logs.read_action') } },
  )

  loading.value = false
  fileText.value = result.ok ? result.value : ''
}

async function refresh() {
  await loadFiles()
  if (!isLive.value) await loadFile(source.value)
}

async function copy() {
  if (!await copyToClipboard(lines.value.map(line => line.text).join('\n'))) {
    toast.add({ title: t('common.copy_failed'), color: 'error', icon: 'i-lucide-clipboard-x' })
    return
  }

  toast.add({ title: t('instance.logs.copied'), color: 'success', icon: 'i-lucide-clipboard-check' })
}

const removing = ref(false)

async function removeFile() {
  if (isLive.value || removing.value) return
  removing.value = true

  const result = await attempt(
    () => call('delete_instance_log', { instanceId: props.instanceId, name: source.value }),
    { context: { instanceId: props.instanceId, action: t('instance.logs.delete_action') } },
  )

  removing.value = false

  if (!result.ok) return

  files.value = result.value
  source.value = LIVE
  fileText.value = ''
}

const openFolder = () => safeRun(
  () => call('open_instance_dir', { instanceId: props.instanceId, target: 'logs' }),
  { context: { instanceId: props.instanceId, action: t('instance.logs.open_dir_action') } },
)

function scrollToEnd() {
  const element = viewer.value
  if (element) element.scrollTop = element.scrollHeight
}

watch(source, async (name) => {
  fileText.value = ''
  if (name !== LIVE) await loadFile(name)
  await nextTick()
  scrollToEnd()
})

watch(() => lines.value.length, async () => {
  if (!autoscroll.value) return
  await nextTick()
  scrollToEnd()
})

watch(() => props.instanceId, () => {
  source.value = LIVE
  fileText.value = ''
  loadFiles()
})

onMounted(async () => {
  await loadFiles()
  await nextTick()
  scrollToEnd()
})
</script>

<template>
  <KitPanel
    index="01"
    :title="$t('instance.logs.title')"
    icon="i-lucide-scroll-text"
  >
    <div class="space-y-5">
      <div class="flex flex-wrap items-end gap-4">
        <KitField
          :label="$t('instance.logs.source')"
          class="min-w-64 flex-1"
        >
          <KitSelect
            v-model="source"
            :items="sources"
          />
        </KitField>

        <KitField
          :label="$t('instance.logs.search')"
          class="min-w-48 flex-1"
        >
          <KitSearchInput
            v-model="filter"
            :placeholder="$t('instance.logs.search_placeholder')"
            font="mono"
          />
        </KitField>

        <div class="flex items-center gap-3">
          <Button
            icon="i-lucide-refresh-cw"
            :loading="loading"
            @click="refresh"
          >
            {{ $t('instance.logs.refresh') }}
          </Button>

          <Button
            icon="i-lucide-copy"
            :disabled="!lines.length"
            @click="copy"
          >
            {{ $t('instance.logs.copy') }}
          </Button>

          <Button
            icon="i-lucide-folder-clock"
            @click="openFolder"
          >
            {{ $t('instance.logs.folder') }}
          </Button>

          <Button
            v-if="isLive"
            variant="quiet"
            icon="i-lucide-eraser"
            :disabled="!live.length"
            @click="instanceStore.clearLogs(props.instanceId)"
          >
            {{ $t('instance.logs.clear') }}
          </Button>

          <Button
            v-else
            variant="quiet-danger"
            icon="i-lucide-trash-2"
            :loading="removing"
            @click="removeFile"
          >
            {{ $t('instance.logs.delete_file') }}
          </Button>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-between gap-4 border-t border-line pt-4">
        <p class="font-mono text-label uppercase tracking-caps text-fg-faint">
          {{ $t('instance.logs.lines', { count: lines.length }) }}<template v-if="filter.trim()">
            {{ $t('instance.logs.lines_of', { total: all.length }) }}
          </template>
          <template v-if="isLive && running">
            · {{ $t('instance.logs.session_running') }}
          </template>
        </p>

        <div class="flex items-center gap-6">
          <Label class="flex cursor-pointer items-center gap-2.5">
            <Switch v-model="autoscroll" />
            {{ $t('instance.logs.autoscroll') }}
          </Label>

          <Label class="flex cursor-pointer items-center gap-2.5">
            <Switch v-model="wrap" />
            {{ $t('instance.logs.wrap') }}
          </Label>
        </div>
      </div>

      <div
        ref="viewer"
        class="h-[26rem] overflow-auto border border-line bg-ink-900 p-4"
        :class="wrap ? '' : 'whitespace-nowrap'"
      >
        <p
          v-for="(line, i) in lines"
          :key="i"
          class="font-mono text-caption leading-[1.55]"
          :class="[
            wrap ? 'break-words whitespace-pre-wrap' : 'whitespace-pre',
            line.tone === 'error' ? 'text-danger' : line.tone === 'warn' ? 'text-warning' : 'text-fg-muted',
          ]"
        >
          {{ line.text || " " }}
        </p>

        <p
          v-if="!lines.length"
          class="flex h-full items-center justify-center font-mono text-label uppercase tracking-caps text-fg-faint"
        >
          <template v-if="loading">
            {{ $t('instance.logs.loading') }}
          </template>
          <template v-else-if="filter.trim()">
            {{ $t('instance.logs.nothing_found') }}
          </template>
          <template v-else-if="isLive">
            {{ $t('instance.logs.live_empty') }}
          </template>
          <template v-else>
            {{ $t('instance.logs.file_empty') }}
          </template>
        </p>
      </div>
    </div>
  </KitPanel>
</template>
