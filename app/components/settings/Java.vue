<script setup lang="ts">
import type { AppConfig } from '~/types/app'

const config = defineModel<AppConfig | null>()
</script>

<template>
  <KitPanel
    index="03"
    :title="$t('settings.java.title')"
    icon="i-lucide-cpu"
  >
    <div class="space-y-7">
      <SettingsMemory
        v-model:min="config!.java.min_ram"
        v-model:max="config!.java.max_ram"
      />

      <SettingsJavaRuntimes
        v-model:mode="config!.java.java_mode"
        v-model:path="config!.java.java_path"
      />
    </div>
  </KitPanel>
</template>
