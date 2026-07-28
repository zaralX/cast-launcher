<script setup lang="ts">
import {ru} from "#ui/locale";
import type {AppConfig} from "~/types/app";

const config = defineModel<AppConfig | null>()
</script>

<template>
  <SettingsPanel
      index="01"
      title="Лаунчер"
      description="Язык интерфейса, оформление и расположение файлов."
      icon="i-lucide-app-window"
  >
    <div class="space-y-7">
      <div class="grid gap-6 sm:grid-cols-2">
        <SettingsField label="Язык">
          <ULocaleSelect
              v-model="config!.launcher.language"
              :locales="[ru]"
              class="w-full"
          />
        </SettingsField>

        <SettingsField label="Тема">
          <UColorModeSelect class="w-full"/>
        </SettingsField>
      </div>

      <SettingsField label="Файлы лаунчера" hint="Сюда попадают версии, библиотеки, ассеты и сами сборки.">
        <UInput
            v-model="config!.launcher.dir"
            placeholder="/path/to/launcher"
            class="w-full"
            :ui="{ base: 'font-mono text-[12px]' }"
        />
      </SettingsField>

      <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Автообновление</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            Проверять и устанавливать новую версию лаунчера при запуске.
          </p>
        </div>
        <USwitch v-model="config!.launcher.auto_update" size="lg"/>
      </div>
    </div>
  </SettingsPanel>
</template>
