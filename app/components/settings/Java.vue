<script setup lang="ts">
import type { AppConfig } from '~/types/app'

const config = defineModel<AppConfig>({ required: true })
</script>

<template>
  <div class="space-y-8">
    <SettingsJavaRuntimes
      v-model:mode="config.java.java_mode"
      v-model:path="config.java.java_path"
    />

    <section>
      <h3 class="text-title font-semibold text-fg">
        {{ $t('settings.java.memory_title') }}
      </h3>
      <p class="mt-0.5 mb-3 text-caption text-fg-faint">
        {{ $t('settings.java.memory_hint') }}
      </p>
      <SettingsMemory
        v-model:min="config.java.min_ram"
        v-model:max="config.java.max_ram"
        class="max-w-xl"
      />
    </section>
  </div>
</template>
