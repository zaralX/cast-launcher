<script setup lang="ts">
import {ru} from "#ui/locale";
import type {AppConfig} from "~/types/app";

const config = defineModel<AppConfig | null>()
</script>

<template>
  <UPageCard
      title="Лаунчер"
      description="Базовые настройки лаунчера."
      variant="soft"
  >
    <div class="space-y-4">
      <div class="grid grid-cols-3 lg:grid-cols-2">
        <div>
          <label class="text-sm mb-1 block">Язык</label>
          <ULocaleSelect
              v-model="config!.launcher.language"
              :locales="[ru]"
              class="w-48"
          />
        </div>

        <div>
          <label class="text-sm mb-1 block">Тема</label>
          <UColorModeSelect />
        </div>
      </div>

      <div>
        <UFormField label="Файлы лаунчера">
          <UInput
              v-model="config!.launcher.dir"
              placeholder="/path/to/launcher"
              class="w-full"
          />
        </UFormField >
      </div>
    </div>
  </UPageCard>
</template>

<style scoped>

</style>