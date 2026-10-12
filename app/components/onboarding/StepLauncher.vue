<script setup lang="ts">
import type { ImportReport } from '~/types/import'

const model = defineModel<ImportReport | null>({ default: null })

const store = useAppStore()

const autoUpdate = computed({
  get: () => store.config?.launcher.auto_update ?? true,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.auto_update = value
  },
})

const telemetry = computed({
  get: () => store.config?.launcher.telemetry ?? true,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.telemetry = value
    setTelemetryEnabled(value)
  },
})
</script>

<template>
  <OnboardingPane :title="$t('onboarding.launcher.title')">
    <div class="space-y-10">
      <div class="space-y-6">
        <KitRow
          :label="$t('settings.launcher.auto_update.label')"
          :description="$t('settings.launcher.auto_update.hint')"
        >
          <Switch
            v-model="autoUpdate"
            size="lg"
          />
        </KitRow>

        <KitRow
          :label="$t('settings.launcher.telemetry.label')"
          :description="$t('settings.launcher.telemetry.hint')"
        >
          <Switch
            v-model="telemetry"
            size="lg"
          />
        </KitRow>
      </div>

      <div>
        <Label class="mb-5 border-t border-line pt-5">
          {{ $t('settings.import.title') }}
        </Label>

        <ImportWizard @imported="report => model = report" />
      </div>
    </div>
  </OnboardingPane>
</template>
