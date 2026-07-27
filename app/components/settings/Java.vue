<script setup lang="ts">
import type {AppConfig} from "~/types/app";

const config = defineModel<AppConfig | null>()

const gb = (mb?: number) => ((mb ?? 0) / 1024).toFixed(1).replace(".", ",")
</script>

<template>
  <SettingsPanel
      index="03"
      title="Java"
      description="Параметры виртуальной машины, с которыми стартует игра."
      icon="i-lucide-cpu"
  >
    <div class="space-y-7">
      <div class="grid gap-6 sm:grid-cols-2">
        <SettingsField label="Минимум RAM" :hint="`≈ ${gb(config!.java.min_ram)} ГБ`">
          <UInput
              v-model="config!.java.min_ram"
              type="number"
              :min="1"
              class="w-full"
              :ui="{ base: 'font-mono tabular-nums' }"
          >
            <template #trailing>
              <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">MB</span>
            </template>
          </UInput>
        </SettingsField>

        <SettingsField label="Максимум RAM" :hint="`≈ ${gb(config!.java.max_ram)} ГБ`">
          <UInput
              v-model="config!.java.max_ram"
              type="number"
              :min="config!.java.min_ram ?? 1"
              class="w-full"
              :ui="{ base: 'font-mono tabular-nums' }"
          >
            <template #trailing>
              <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">MB</span>
            </template>
          </UInput>
        </SettingsField>
      </div>

      <SettingsField label="Путь к Java" hint="Пусто — будет использована java из PATH.">
        <UInput
            v-model="config!.java.java_path"
            placeholder="/path/to/java"
            class="w-full"
            :ui="{ base: 'font-mono text-[12px]' }"
        />
      </SettingsField>
    </div>
  </SettingsPanel>
</template>
