<script setup lang="ts">
import type { InstanceSettings } from '~/types/instance'

const settings = defineModel<InstanceSettings>({ required: true })

const { config } = storeToRefs(useAppStore())
const { t } = useI18n()

const globalJava = computed(() => config.value?.java ?? null)

const minRam = computed({
  get: () => settings.value.overrideMemory ? settings.value.minRam : globalJava.value?.min_ram ?? 0,
  set: (value: number) => settings.value.minRam = value,
})

const maxRam = computed({
  get: () => settings.value.overrideMemory ? settings.value.maxRam : globalJava.value?.max_ram ?? 0,
  set: (value: number) => settings.value.maxRam = value,
})

const javaMode = computed({
  get: () => settings.value.overrideJava ? settings.value.javaMode : globalJava.value?.java_mode ?? 'auto',
  set: (value: typeof settings.value.javaMode) => settings.value.javaMode = value,
})

const javaPath = computed({
  get: () => settings.value.overrideJava ? settings.value.javaPath : globalJava.value?.java_path ?? '',
  set: (value: string) => settings.value.javaPath = value,
})

const inheritedMemory = computed(() => {
  const java = globalJava.value
  if (!java) return ''
  return `${formatBytes(java.min_ram * 1024 * 1024)} – ${formatBytes(java.max_ram * 1024 * 1024)}`
})

const inheritedJava = computed(() => {
  const java = globalJava.value
  if (!java) return ''
  if (java.java_mode === 'manual' && java.java_path.trim()) return java.java_path
  return java.java_mode === 'system' ? t('settings.java.system') : t('settings.java.auto')
})

const inheritedJvmArgs = computed(() => globalJava.value?.jvm_args.trim() ?? '')

function toggleJvmArgs(enabled: boolean) {
  settings.value.overrideJvmArgs = enabled
  if (enabled && !settings.value.jvmArgs.trim()) settings.value.jvmArgs = inheritedJvmArgs.value
}

watch(() => settings.value.overrideMemory, (enabled) => {
  if (!enabled) return
  if (!settings.value.minRam) settings.value.minRam = globalJava.value?.min_ram ?? 1024
  if (!settings.value.maxRam) settings.value.maxRam = globalJava.value?.max_ram ?? 4096
})

watch(() => settings.value.overrideJava, (enabled) => {
  if (!enabled) return
  if (settings.value.javaMode === 'auto' && !settings.value.javaPath.trim()) {
    settings.value.javaMode = globalJava.value?.java_mode ?? 'auto'
    settings.value.javaPath = globalJava.value?.java_path ?? ''
  }
})
</script>

<template>
  <div class="flex flex-col gap-3.5">
    <KitPanel
      :title="$t('instance.java.memory_title')"
    >
      <template #action>
        <KitOverride v-model="settings.overrideMemory" />
      </template>

      <SettingsMemory
        v-if="settings.overrideMemory"
        v-model:min="minRam"
        v-model:max="maxRam"
      />
      <KitInherited v-else>
        {{ inheritedMemory }} · {{ $t('instance.java.from_launcher') }}
      </KitInherited>
    </KitPanel>

    <KitPanel
      :title="$t('instance.java.title')"
    >
      <template #action>
        <KitOverride v-model="settings.overrideJava" />
      </template>

      <SettingsJavaRuntimes
        v-if="settings.overrideJava"
        v-model:mode="javaMode"
        v-model:path="javaPath"
      />
      <KitInherited
        v-else
        :mono="globalJava?.java_mode === 'manual'"
      >
        {{ inheritedJava }} · {{ $t('instance.java.from_launcher') }}
      </KitInherited>
    </KitPanel>

    <KitPanel
      :title="$t('instance.java.args_title')"
    >
      <template #action>
        <KitOverride
          :model-value="settings.overrideJvmArgs"
          @update:model-value="toggleJvmArgs"
        />
      </template>

      <SettingsJvmArgs
        v-if="settings.overrideJvmArgs"
        v-model="settings.jvmArgs"
      />
      <KitInherited
        v-else
        :mono="!!inheritedJvmArgs"
      >
        {{ inheritedJvmArgs || $t('instance.java.no_args') }} · {{ $t('instance.java.from_launcher') }}
      </KitInherited>
    </KitPanel>
  </div>
</template>
