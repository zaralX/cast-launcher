<script setup lang="ts">
import {setTelemetryEnabled} from "~/composables/useTelemetry"
import {useAppStore} from "~/stores/app"
import type {ImportReport} from "~/types/import"

const model = defineModel<ImportReport | null>({default: null})

const store = useAppStore()

const autoUpdate = computed({
  get: () => store.config?.launcher.auto_update ?? true,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.auto_update = value
  }
})

const telemetry = computed({
  get: () => store.config?.launcher.telemetry ?? true,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.telemetry = value
    setTelemetryEnabled(value)
  }
})
</script>

<template>
  <OnboardingPane :title="$t('onboarding.launcher.title')">
    <div class="space-y-10">
      <div>
        <div class="flex items-center justify-between gap-6 border-t border-line py-5">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ $t('settings.launcher.auto_update.label') }}
            </p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              {{ $t('settings.launcher.auto_update.hint') }}
            </p>
          </div>
          <USwitch v-model="autoUpdate" size="lg"/>
        </div>

        <div class="flex items-center justify-between gap-6 border-t border-line py-5">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ $t('settings.launcher.telemetry.label') }}
            </p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              {{ $t('settings.launcher.telemetry.hint') }}
            </p>
          </div>
          <USwitch v-model="telemetry" size="lg"/>
        </div>
      </div>

      <div>
        <p class="mb-5 border-t border-line pt-5 font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
          {{ $t('settings.import.title') }}
        </p>

        <ImportWizard @imported="report => model = report"/>
      </div>
    </div>
  </OnboardingPane>
</template>
