<script setup lang="ts">
import type { InstanceLogFile } from '~/types/instance'

const props = defineProps<{ instanceId: string }>()

const LIVE = 'live'

const { t, locale } = useI18n()
const instanceStore = useInstanceStore()
const { logs } = storeToRefs(instanceStore)
const toast = useToast()

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
  <SettingsPanel
    index="01"
    :title="$t('instance.logs.title')"
    icon="i-lucide-scroll-text"
  >
    <div class="space-y-5">
      <div class="flex flex-wrap items-end gap-4">
        <SettingsField
          :label="$t('instance.logs.source')"
          class="min-w-[16rem] flex-1"
        >
          <USelect
            v-model="source"
            :items="sources"
            class="w-full"
          />
        </SettingsField>

        <SettingsField
          :label="$t('instance.logs.search')"
          class="min-w-[12rem] flex-1"
        >
          <UInput
            v-model="filter"
            :placeholder="$t('instance.logs.search_placeholder')"
            class="w-full"
            :ui="{ base: 'font-mono text-[12px]' }"
          >
            <template #trailing>
              <UIcon
                name="i-lucide-search"
                class="size-3.5 text-fg-faint"
              />
            </template>
          </UInput>
        </SettingsField>

        <div class="flex items-center gap-3 pb-1">
          <AppButton
            class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
            icon="i-lucide-refresh-cw"
            :loading="loading"
            @click="refresh"
          >
            {{ $t('instance.logs.refresh') }}
          </AppButton>

          <AppButton
            class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
            icon="i-lucide-copy"
            :disabled="!lines.length"
            @click="copy"
          >
            {{ $t('instance.logs.copy') }}
          </AppButton>

          <AppButton
            class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
            icon="i-lucide-folder-clock"
            @click="openFolder"
          >
            {{ $t('instance.logs.folder') }}
          </AppButton>

          <AppButton
            v-if="isLive"
            tone="quiet"
            class="text-[10px] tracking-[0.18em]"
            icon="i-lucide-eraser"
            :disabled="!live.length"
            @click="instanceStore.clearLogs(props.instanceId)"
          >
            {{ $t('instance.logs.clear') }}
          </AppButton>

          <AppButton
            v-else
            tone="quiet"
            class="text-[10px] tracking-[0.18em] hover:text-red-400"
            icon="i-lucide-trash-2"
            :loading="removing"
            @click="removeFile"
          >
            {{ $t('instance.logs.delete_file') }}
          </AppButton>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-between gap-4 border-t border-line pt-4">
        <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
          {{ $t('instance.logs.lines', { count: lines.length }) }}<template v-if="filter.trim()">
            {{ $t('instance.logs.lines_of', { total: all.length }) }}
          </template>
          <template v-if="isLive && running">
            · {{ $t('instance.logs.session_running') }}
          </template>
        </p>

        <div class="flex items-center gap-6">
          <div class="flex items-center gap-2.5">
            <USwitch v-model="autoscroll" />
            <span class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">{{ $t('instance.logs.autoscroll') }}</span>
          </div>

          <div class="flex items-center gap-2.5">
            <USwitch v-model="wrap" />
            <span class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">{{ $t('instance.logs.wrap') }}</span>
          </div>
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
          class="font-mono text-[11px] leading-[1.55]"
          :class="[
            wrap ? 'whitespace-pre-wrap break-words' : 'whitespace-pre',
            line.tone === 'error' ? 'text-red-400' : line.tone === 'warn' ? 'text-amber-400' : 'text-fg-muted',
          ]"
        >
          {{ line.text || " " }}
        </p>

        <p
          v-if="!lines.length"
          class="flex justify-center items-center h-full font-mono text-xs uppercase tracking-[0.24em] text-fg-faint"
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
  </SettingsPanel>
</template>
