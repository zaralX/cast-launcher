<script setup lang="ts">
import {ru} from "#ui/locale";
import type {AppConfig} from "~/types/app";
import {ACCENTS} from "~/composables/useAppearance";

const config = defineModel<AppConfig | null>()
</script>

<template>
  <SettingsPanel
      index="01"
      title="Лаунчер"
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

      <SettingsField label="Основной цвет" hint="Меняется сразу, но сохраняется только по кнопке.">
        <div class="flex flex-wrap gap-2">
          <button
              v-for="accent in ACCENTS"
              :key="accent.value"
              type="button"
              :title="accent.label"
              :aria-label="accent.label"
              :aria-pressed="config!.launcher.accent === accent.value"
              class="group grid size-8 cursor-pointer place-items-center border transition-colors duration-300"
              :class="config!.launcher.accent === accent.value
                ? 'border-fg'
                : 'border-line hover:border-line-strong'"
              @click="config!.launcher.accent = accent.value"
          >
            <span
                class="size-4 transition-transform duration-300 ease-deck group-hover:scale-110"
                :style="{ backgroundColor: accent.preview }"
            />
          </button>
        </div>
      </SettingsField>

      <SettingsField label="Файлы лаунчера" hint="Сюда попадают все файлы связанные с игрой.">
        <UInput
            v-model="config!.launcher.dir"
            placeholder="/path/to/launcher"
            class="w-full"
            :ui="{ base: 'font-mono text-[12px]' }"
        />
      </SettingsField>

      <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Компактный режим</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            Главная страница почти как в Prism Launcher.
          </p>
        </div>
        <USwitch v-model="config!.launcher.compact" size="lg"/>
      </div>

      <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Автообновление</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            Проверять и устанавливать новую версию лаунчера при запуске.
          </p>
        </div>
        <USwitch v-model="config!.launcher.auto_update" size="lg"/>
      </div>

      <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Анонимная статистика</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            Помогает чинить вылеты и ошибки установки. Ник, пути и логи не отправляются.
          </p>
        </div>
        <USwitch v-model="config!.launcher.telemetry" size="lg"/>
      </div>
    </div>
  </SettingsPanel>
</template>
