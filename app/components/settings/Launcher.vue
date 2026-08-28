<script setup lang="ts">
import type {AppConfig} from "~/types/app";
import {AFTER_LAUNCH_VALUES} from "~/types/app";
import {ACCENTS} from "~/composables/useAppearance";
import {call} from "~/types/backend";

const config = defineModel<AppConfig | null>()

const {t} = useI18n()

const locales = useAvailableLocales()

const afterLaunchOptions = computed(() => AFTER_LAUNCH_VALUES.map(value => ({
  value,
  label: t(`settings.after_launch.${value}.label`),
  hint: t(`settings.after_launch.${value}.hint`)
})))

const afterLaunchHint = computed(() =>
    afterLaunchOptions.value.find(option => option.value === config.value?.launcher.after_launch)?.hint
)

async function pickLauncherDir() {
  const picked = await safeRun(() => call("pick_folder", {
    title: t("settings.launcher.dir.label"),
    directory: config.value?.launcher.dir
  }))

  if (picked) config.value!.launcher.dir = picked
}
</script>

<template>
  <SettingsPanel
      index="01"
      :title="$t('settings.launcher.title')"
      icon="i-lucide-app-window"
  >
    <div class="space-y-7">
      <div class="grid gap-6 sm:grid-cols-2">
        <SettingsField :label="$t('settings.launcher.language')">
          <ULocaleSelect
              v-model="config!.launcher.language"
              :locales="locales"
              class="w-full"
          />
        </SettingsField>

        <SettingsField :label="$t('settings.launcher.theme')">
          <UColorModeSelect class="w-full"/>
        </SettingsField>
      </div>

      <SettingsField :label="$t('settings.launcher.accent.label')" :hint="$t('settings.launcher.accent.hint')">
        <div class="flex flex-wrap gap-2">
          <button
              v-for="accent in ACCENTS"
              :key="accent.value"
              type="button"
              :title="$t(accent.labelKey)"
              :aria-label="$t(accent.labelKey)"
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

      <SettingsField :label="$t('settings.launcher.after_launch.label')" :hint="afterLaunchHint">
        <USelect
            v-model="config!.launcher.after_launch"
            :items="afterLaunchOptions"
            value-key="value"
            class="w-full sm:w-1/2"
        />
      </SettingsField>

      <SettingsField
          :label="$t('settings.launcher.dir.label')"
          :hint="$t('settings.launcher.dir.hint')"
      >
        <div class="flex gap-2">
          <UInput
              v-model="config!.launcher.dir"
              placeholder="/path/to/launcher"
              class="w-full"
              :ui="{ base: 'font-mono text-[12px]' }"
          />

          <AppButton
              class="h-9 shrink-0 px-4 text-[10px] tracking-[0.18em]"
              icon="i-lucide-folder-open"
              @click="pickLauncherDir"
          >
            {{ $t('common.pick') }}
          </AppButton>
        </div>
      </SettingsField>

      <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ $t('settings.launcher.compact.label') }}</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            {{ $t('settings.launcher.compact.hint') }}
          </p>
        </div>
        <USwitch v-model="config!.launcher.compact" size="lg"/>
      </div>

      <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ $t('settings.launcher.auto_update.label') }}</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            {{ $t('settings.launcher.auto_update.hint') }}
          </p>
        </div>
        <USwitch v-model="config!.launcher.auto_update" size="lg"/>
      </div>

      <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
        <div class="min-w-0">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ $t('settings.launcher.telemetry.label') }}</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            {{ $t('settings.launcher.telemetry.hint') }}
          </p>
        </div>
        <USwitch v-model="config!.launcher.telemetry" size="lg"/>
      </div>
    </div>
  </SettingsPanel>
</template>
