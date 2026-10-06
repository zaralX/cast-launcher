<script setup lang="ts">
import type { InstanceSettings } from '~/types/instance'

const settings = defineModel<InstanceSettings>({ required: true })

const { config } = storeToRefs(useAppStore())

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
  <div class="space-y-6">
    <KitPanel
      index="01"
      :title="$t('instance.java.memory_title')"
      icon="i-lucide-memory-stick"
    >
      <div class="space-y-7">
        <KitRow
          :bordered="false"
          :label="$t('instance.java.override_memory')"
          :description="$t('instance.java.override_memory_hint')"
        >
          <Switch
            v-model="settings.overrideMemory"
            size="lg"
          />
        </KitRow>

        <SettingsMemory
          v-model:min="minRam"
          v-model:max="maxRam"
          :disabled="!settings.overrideMemory"
          class="border-t border-line pt-6 transition-opacity duration-300"
          :class="settings.overrideMemory ? '' : 'opacity-45'"
        />
      </div>
    </KitPanel>

    <KitPanel
      index="02"
      :title="$t('instance.java.title')"
      :description="$t('instance.java.description')"
      icon="i-lucide-cpu"
    >
      <div class="space-y-7">
        <KitRow
          :bordered="false"
          :label="$t('instance.java.override_java')"
          :description="$t('instance.java.override_java_hint')"
        >
          <Switch
            v-model="settings.overrideJava"
            size="lg"
          />
        </KitRow>

        <div
          class="border-t border-line pt-6 transition-opacity duration-300"
          :class="settings.overrideJava ? '' : 'pointer-events-none opacity-45'"
        >
          <SettingsJavaRuntimes
            v-model:mode="javaMode"
            v-model:path="javaPath"
          />
        </div>
      </div>
    </KitPanel>
  </div>
</template>
