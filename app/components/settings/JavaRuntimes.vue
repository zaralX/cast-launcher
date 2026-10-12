<script setup lang="ts">
import { JAVA_SOURCE_KEYS } from '~/types/app'
import type { JavaMode, JavaRuntime } from '~/types/app'

const mode = defineModel<JavaMode>('mode', { required: true })
const path = defineModel<string>('path', { required: true })

const { t } = useI18n()
const store = useAppStore()
const { javaRuntimes, javaScanning } = storeToRefs(store)

const setMode = (next: JavaMode, nextPath = '') => {
  mode.value = next
  path.value = nextPath
}

const javaPath = computed({
  get: () => path.value ?? '',
  set: (value: string) => setMode(value.trim() ? 'manual' : 'auto', value),
})

const isManual = computed(() => mode.value === 'manual' && !!javaPath.value.trim())
const isDetected = computed(() => javaRuntimes.value.some(r => r.path === javaPath.value))

const majors = computed(() => [...new Set(javaRuntimes.value.map(r => r.major))].sort((a, b) => a - b))

const choice = computed({
  get: () => mode.value,
  set: (value: JavaMode) => {
    if (value === 'manual') mode.value = 'manual'
    else setMode(value)
  },
})

const modeHint = computed(() => {
  if (mode.value === 'manual') return ''
  if (javaScanning.value && !javaRuntimes.value.length) return t('settings.java.scanning')

  if (mode.value === 'system') {
    const runtime = store.systemJavaRuntime
    return runtime ? `Java ${runtime.major} · ${runtime.vendor}` : t('settings.java.not_found')
  }

  return majors.value.length ? t('settings.java.auto_hint', { majors: majors.value.join(', ') }) : t('settings.java.not_found')
})

const sourceOf = (runtime: JavaRuntime) =>
  JAVA_SOURCE_KEYS[runtime.source] ? t(JAVA_SOURCE_KEYS[runtime.source]!) : runtime.source

const select = (runtime: JavaRuntime) => setMode('manual', runtime.path)

const rescan = () => safeRun(() => store.scanJava(true))

const manual = ref<JavaRuntime | null>(null)
const manualChecking = ref(false)
let probeId = 0
let probeTimer: ReturnType<typeof setTimeout> | null = null

watch([javaPath, javaRuntimes], () => {
  if (probeTimer) clearTimeout(probeTimer)

  const value = javaPath.value.trim()
  if (!isManual.value || isDetected.value) {
    manual.value = null
    manualChecking.value = false
    return
  }

  manualChecking.value = true
  const id = ++probeId

  probeTimer = setTimeout(async () => {
    const result = await store.probeJava(value).catch(() => null)

    if (id !== probeId) return
    manual.value = result
    manualChecking.value = false
  }, 400)
}, { immediate: true })

onMounted(() => safeRun(() => store.scanJava()))
onBeforeUnmount(() => {
  if (probeTimer) clearTimeout(probeTimer)
})
</script>

<template>
  <div class="space-y-6">
    <div>
      <Label class="mb-1.5">
        {{ $t('settings.java.mode_label') }}
      </Label>
      <div class="flex flex-wrap items-center gap-3.5">
        <RadioGroup
          v-model="choice"
          :aria-label="$t('settings.java.mode_label')"
          class="w-fit"
        >
          <RadioGroupItem
            value="auto"
            class="flex-none"
          >
            {{ $t('settings.java.auto') }}
          </RadioGroupItem>
          <RadioGroupItem
            value="system"
            class="flex-none"
          >
            {{ $t('settings.java.system') }}
          </RadioGroupItem>
          <RadioGroupItem
            value="manual"
            class="flex-none"
          >
            {{ $t('settings.java.manual') }}
          </RadioGroupItem>
        </RadioGroup>
        <span class="text-caption text-fg-faint">{{ modeHint }}</span>
      </div>
    </div>

    <KitField
      v-if="mode === 'manual'"
      :label="$t('settings.java.custom_path')"
      for="java-custom-path"
    >
      <Input
        id="java-custom-path"
        v-model="javaPath"
        :placeholder="$t('settings.java.custom_path')"
        font="mono"
      />

      <KitStatus
        v-if="isManual && !isDetected"
        :tone="manualChecking ? 'muted' : manual ? 'success' : 'warning'"
        :dot="manualChecking ? 'live' : 'static'"
        class="mt-2"
      >
        <template v-if="manualChecking">
          {{ $t('settings.java.probing') }}
        </template>
        <template v-else-if="manual">
          Java {{ manual.version }} · {{ manual.vendor }} · {{ manual.arch }}
        </template>
        <template v-else>
          {{ $t('settings.java.probe_failed') }}
        </template>
      </KitStatus>
    </KitField>

    <div>
      <div class="mb-2 flex items-center gap-2">
        <Label>{{ $t('settings.java.found') }}</Label>
        <span class="text-caption text-fg-faint tabular-nums">{{ javaRuntimes.length || '' }}</span>
        <span class="flex-1" />
        <Button
          variant="ghost"
          size="sm"
          icon="i-lucide-refresh-cw"
          :loading="javaScanning"
          @click="rescan"
        >
          {{ javaScanning ? $t('settings.java.searching') : $t('settings.java.refresh') }}
        </Button>
      </div>

      <ul
        v-if="javaRuntimes.length"
        class="max-h-80 divide-y divide-line overflow-y-auto rounded-md ring-1 ring-line ring-inset"
      >
        <KitListItem
          v-for="runtime in javaRuntimes"
          :key="runtime.path"
          :active="isManual && javaPath === runtime.path"
          :active-label="$t('settings.java.active')"
          class="rounded-none px-4"
          @select="select(runtime)"
        >
          <template #leading>
            <span class="w-8 shrink-0 text-center font-mono text-heading text-fg-muted tabular-nums">
              {{ runtime.major }}
            </span>
          </template>

          <p class="flex min-w-0 items-center gap-2 text-fg">
            <span class="truncate">
              {{ runtime.version }} · {{ runtime.vendor }} · {{ runtime.is_64bit ? $t('settings.java.arch64') : $t('settings.java.arch32') }}
            </span>
            <Badge
              v-if="!runtime.is_64bit"
              variant="warning"
            >
              {{ $t('settings.java.bit32') }}
            </Badge>
          </p>
          <p class="truncate font-mono text-micro text-fg-faint">
            {{ runtime.path }}
          </p>

          <template #trailing>
            <Badge>{{ sourceOf(runtime) }}</Badge>
          </template>
        </KitListItem>
      </ul>

      <KitEmpty
        v-else
        icon="i-lucide-coffee"
        class="py-8"
      >
        {{ javaScanning ? $t('settings.java.scanning') : $t('settings.java.empty') }}
      </KitEmpty>
    </div>
  </div>
</template>
