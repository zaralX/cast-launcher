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

const describe = (runtime: JavaRuntime) => [
  runtime.vendor,
  runtime.is_64bit ? t('settings.java.arch64') : t('settings.java.arch32'),
  JAVA_SOURCE_KEYS[runtime.source] ? t(JAVA_SOURCE_KEYS[runtime.source]!) : runtime.source,
].join(' · ')

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
  <div class="space-y-7">
    <div>
      <div class="mb-2 flex items-center justify-between gap-4">
        <Label>{{ $t('settings.java.runtime') }}</Label>

        <Button
          variant="quiet"
          icon="i-lucide-refresh-cw"
          :loading="javaScanning"
          @click="rescan"
        >
          {{ javaScanning ? $t('settings.java.searching') : $t('settings.java.refresh') }}
        </Button>
      </div>

      <ul class="max-h-64 overflow-y-scroll border border-line">
        <KitListItem
          :active="mode === 'auto'"
          :active-label="$t('settings.java.active')"
          @select="setMode('auto')"
        >
          <template #leading>
            <Icon
              name="i-lucide-wand-sparkles"
              class="size-4 shrink-0 text-fg-faint"
            />
          </template>

          <p class="truncate text-title text-fg">
            {{ $t('settings.java.auto') }}
          </p>
          <p class="mt-1 truncate font-mono text-micro uppercase tracking-caps text-fg-faint">
            <template v-if="majors.length">
              {{ $t('settings.java.auto_hint', { majors: majors.join(', ') }) }}
            </template>
            <template v-else-if="javaScanning">
              {{ $t('settings.java.scanning') }}
            </template>
            <template v-else>
              {{ $t('settings.java.not_found') }}
            </template>
          </p>
        </KitListItem>

        <KitListItem
          :active="mode === 'system'"
          :active-label="$t('settings.java.active')"
          @select="setMode('system')"
        >
          <template #leading>
            <Icon
              name="i-lucide-square-terminal"
              class="size-4 shrink-0 text-fg-faint"
            />
          </template>

          <p class="truncate text-title text-fg">
            {{ $t('settings.java.system') }}
          </p>
          <p class="mt-1 truncate font-mono text-micro uppercase tracking-caps text-fg-faint">
            <template v-if="store.systemJavaRuntime">
              Java {{ store.systemJavaRuntime.major }} · {{ store.systemJavaRuntime.vendor }}
            </template>
            <template v-else-if="javaScanning">
              {{ $t('settings.java.scanning') }}
            </template>
            <template v-else>
              {{ $t('settings.java.not_found') }}
            </template>
          </p>
        </KitListItem>

        <KitListItem
          v-for="runtime in javaRuntimes"
          :key="runtime.path"
          :active="isManual && javaPath === runtime.path"
          :active-label="$t('settings.java.active')"
          @select="select(runtime)"
        >
          <template #leading>
            <span class="shrink-0 font-mono text-heading leading-none tabular-nums text-fg-muted">
              {{ runtime.major }}
            </span>
          </template>

          <p class="truncate text-title text-fg">
            Java {{ runtime.version }}
            <span
              v-if="!runtime.is_64bit"
              class="ml-2 font-mono text-micro uppercase tracking-caps text-warning"
            >
              {{ $t('settings.java.bit32') }}
            </span>
          </p>
          <p class="mt-1 truncate font-mono text-micro uppercase tracking-caps text-fg-faint">
            {{ describe(runtime) }}
          </p>
          <p class="mt-1 truncate font-mono text-label text-fg-faint/70">
            {{ runtime.path }}
          </p>
        </KitListItem>
      </ul>

      <p
        v-if="!javaRuntimes.length && !javaScanning"
        class="border-b border-line py-6 text-center font-mono text-label uppercase tracking-caps text-fg-faint"
      >
        {{ $t('settings.java.empty') }}
      </p>
    </div>

    <KitField
      :label="$t('settings.java.custom_path')"
      :hint="$t('settings.java.custom_path_hint')"
      for="java-custom-path"
    >
      <Input
        id="java-custom-path"
        v-model="javaPath"
        :placeholder="mode === 'system' ? $t('settings.java.system') : $t('settings.java.auto')"
        font="mono"
      />

      <p
        v-if="isManual && !isDetected"
        class="mt-2 flex items-center gap-2 font-mono text-label tracking-[0.02em]"
      >
        <template v-if="manualChecking">
          <Spinner class="size-3 text-fg-faint" />
          <span class="text-fg-faint">{{ $t('settings.java.probing') }}</span>
        </template>
        <template v-else-if="manual">
          <Icon
            name="i-lucide-check"
            class="size-3 shrink-0 text-acid"
          />
          <span class="text-fg-muted">Java {{ manual.version }} · {{ manual.vendor }} · {{ manual.arch }}</span>
        </template>
        <template v-else>
          <Icon
            name="i-lucide-triangle-alert"
            class="size-3 shrink-0 text-warning"
          />
          <span class="text-warning">{{ $t('settings.java.probe_failed') }}</span>
        </template>
      </p>
    </KitField>
  </div>
</template>
