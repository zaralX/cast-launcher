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
        <span class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ $t('settings.java.runtime') }}</span>

        <AppButton
          tone="quiet"
          class="text-[10px] tracking-[0.18em]"
          icon="i-lucide-refresh-cw"
          :loading="javaScanning"
          @click="rescan"
        >
          {{ javaScanning ? $t('settings.java.searching') : $t('settings.java.refresh') }}
        </AppButton>
      </div>

      <ul class="border border-line max-h-64 overflow-y-scroll">
        <li
          class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 pr-1 transition-colors duration-300 hover:bg-ink-700"
          @click="setMode('auto')"
        >
          <span
            class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
            :class="mode === 'auto' ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
          />

          <UIcon
            name="i-lucide-wand-sparkles"
            class="size-4 shrink-0 text-fg-faint"
          />

          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] text-fg">
              {{ $t('settings.java.auto') }}
            </p>
            <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
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
          </div>

          <span
            v-if="mode === 'auto'"
            class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-acid"
          >
            {{ $t('settings.java.active') }}
          </span>
        </li>

        <li
          class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 pr-1 transition-colors duration-300 hover:bg-ink-700"
          @click="setMode('system')"
        >
          <span
            class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
            :class="mode === 'system' ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
          />

          <UIcon
            name="i-lucide-square-terminal"
            class="size-4 shrink-0 text-fg-faint"
          />

          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] text-fg">
              {{ $t('settings.java.system') }}
            </p>
            <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
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
          </div>

          <span
            v-if="mode === 'system'"
            class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-acid"
          >
            {{ $t('settings.java.active') }}
          </span>
        </li>

        <li
          v-for="runtime in javaRuntimes"
          :key="runtime.path"
          class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 pr-1 transition-colors duration-300 hover:bg-ink-700"
          @click="select(runtime)"
        >
          <span
            class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
            :class="isManual && javaPath === runtime.path ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
          />

          <span class="shrink-0 font-mono text-[15px] leading-none tabular-nums text-fg-muted">
            {{ runtime.major }}
          </span>

          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] text-fg">
              Java {{ runtime.version }}
              <span
                v-if="!runtime.is_64bit"
                class="ml-2 font-mono text-[9px] uppercase tracking-[0.2em] text-amber-400"
              >
                {{ $t('settings.java.bit32') }}
              </span>
            </p>
            <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ describe(runtime) }}
            </p>
            <p class="mt-1 truncate font-mono text-[10px] text-fg-faint/70">
              {{ runtime.path }}
            </p>
          </div>

          <span
            v-if="isManual && javaPath === runtime.path"
            class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-acid"
          >
            {{ $t('settings.java.active') }}
          </span>
        </li>
      </ul>

      <p
        v-if="!javaRuntimes.length && !javaScanning"
        class="border-b border-line py-6 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
      >
        {{ $t('settings.java.empty') }}
      </p>
    </div>

    <SettingsField
      :label="$t('settings.java.custom_path')"
      :hint="$t('settings.java.custom_path_hint')"
    >
      <UInput
        v-model="javaPath"
        :placeholder="mode === 'system' ? $t('settings.java.system') : $t('settings.java.auto')"
        class="w-full"
        :ui="{ base: 'font-mono text-[12px]' }"
      />

      <p
        v-if="isManual && !isDetected"
        class="mt-2 flex items-center gap-2 font-mono text-[10px] tracking-[0.02em]"
      >
        <template v-if="manualChecking">
          <UIcon
            name="i-lucide-loader-circle"
            class="size-3 shrink-0 animate-spin text-fg-faint"
          />
          <span class="text-fg-faint">{{ $t('settings.java.probing') }}</span>
        </template>
        <template v-else-if="manual">
          <UIcon
            name="i-lucide-check"
            class="size-3 shrink-0 text-acid"
          />
          <span class="text-fg-muted">Java {{ manual.version }} · {{ manual.vendor }} · {{ manual.arch }}</span>
        </template>
        <template v-else>
          <UIcon
            name="i-lucide-triangle-alert"
            class="size-3 shrink-0 text-amber-400"
          />
          <span class="text-amber-400">{{ $t('settings.java.probe_failed') }}</span>
        </template>
      </p>
    </SettingsField>
  </div>
</template>
