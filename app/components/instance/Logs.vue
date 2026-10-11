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
  no: number
  text: string
  tone: '' | 'error' | 'warn'
}

const all = computed<Line[]>(() => {
  if (isLive.value) {
    return live.value.map((line, index) => ({ no: index + 1, text: line.line, tone: line.isError ? 'error' : tone(line.line) }))
  }

  const text = fileText.value.replace(/\n$/, '')
  return text ? text.split(/\r?\n/).map((line, index) => ({ no: index + 1, text: line, tone: tone(line) })) : []
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
  <div class="flex flex-col gap-3">
    <div class="flex items-center gap-2">
      <KitSelect
        v-model="source"
        :items="sources"
        :aria-label="$t('instance.logs.source')"
        class="w-64"
      />

      <KitSearchInput
        v-model="filter"
        :placeholder="$t('instance.logs.search_placeholder')"
        :aria-label="$t('instance.logs.search')"
        font="mono"
        class="w-60"
      />

      <span class="flex-1" />

      <Button
        variant="ghost"
        size="icon"
        icon="i-lucide-arrow-down-to-line"
        :aria-pressed="autoscroll"
        :aria-label="$t('instance.logs.autoscroll')"
        :title="$t('instance.logs.autoscroll')"
        @click="autoscroll = !autoscroll"
      />
      <Button
        variant="ghost"
        size="icon"
        icon="i-lucide-wrap-text"
        :aria-pressed="wrap"
        :aria-label="$t('instance.logs.wrap')"
        :title="$t('instance.logs.wrap')"
        @click="wrap = !wrap"
      />
      <Button
        size="icon"
        icon="i-lucide-refresh-cw"
        :loading="loading"
        :aria-label="$t('instance.logs.refresh')"
        :title="$t('instance.logs.refresh')"
        @click="refresh"
      />
      <Button
        size="icon"
        icon="i-lucide-copy"
        :disabled="!lines.length"
        :aria-label="$t('instance.logs.copy')"
        :title="$t('instance.logs.copy')"
        @click="copy"
      />
      <Button
        icon="i-lucide-folder-clock"
        @click="openFolder"
      >
        {{ $t('instance.logs.folder') }}
      </Button>
      <Button
        v-if="isLive"
        variant="ghost"
        icon="i-lucide-eraser"
        :disabled="!live.length"
        @click="instanceStore.clearLogs(props.instanceId)"
      >
        {{ $t('instance.logs.clear') }}
      </Button>
      <Button
        v-else
        variant="danger-ghost"
        icon="i-lucide-trash-2"
        :loading="removing"
        @click="removeFile"
      >
        {{ $t('instance.logs.delete_file') }}
      </Button>
    </div>

    <div
      ref="viewer"
      role="log"
      :aria-label="$t('instance.logs.title')"
      class="h-[calc(100vh-14rem)] min-h-80 overflow-auto rounded-md bg-frame py-2 font-mono text-caption leading-5.5 ring-1 ring-line"
    >
      <div
        v-for="line in lines"
        :key="line.no"
        class="flex gap-3 px-3.5"
        :class="[
          line.tone === 'error' ? 'bg-danger/12 text-danger' : line.tone === 'warn' ? 'text-warning' : 'text-fg-muted',
          wrap ? '' : 'w-max min-w-full',
        ]"
      >
        <span class="w-10 shrink-0 text-right text-fg-faint/70 select-none tabular-nums">{{ line.no }}</span>
        <span :class="wrap ? 'min-w-0 break-words whitespace-pre-wrap' : 'whitespace-pre'">{{ line.text || " " }}</span>
      </div>

      <p
        v-if="!lines.length"
        class="flex h-full items-center justify-center font-sans text-body text-fg-faint"
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

    <p class="text-caption text-fg-faint">
      {{ $t('instance.logs.lines', { count: lines.length }) }}<template v-if="filter.trim()">
        {{ $t('instance.logs.lines_of', { total: all.length }) }}
      </template>
      <template v-if="isLive && running">
        · {{ $t('instance.logs.session_running') }}
      </template>
    </p>
  </div>
</template>
