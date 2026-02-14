<script setup lang="ts">
import type {AppConfig} from "~/types/app";

const config = defineModel<AppConfig | null>()
</script>

<template>
  <UPageCard
      title="Java"
      description="Параметры виртуальной машины Java."
      variant="soft"
  >
    <div class="space-y-4">
      <div class="grid grid-cols-3 lg:grid-cols-2">
        <UFormField  label="Минимум RAM (MB)">
          <UInput
              type="number"
              :min="1"
              v-model="config!.java.min_ram"
          />
        </UFormField >

        <UFormField  label="Максимум RAM (MB)">
          <UInput
              type="number"
              :min="config!.java.min_ram ?? 1"
              v-model="config!.java.max_ram"
          />
        </UFormField >
      </div>
      <div>
        <UFormField  label="Путь к Java">
          <UInput
              v-model="config!.java.java_path"
              placeholder="/path/to/java"
              class="w-full"
          />
        </UFormField >
      </div>
    </div>
  </UPageCard>
</template>

<style scoped>

</style>