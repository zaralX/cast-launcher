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
    <div class="space-y-8">
      <KitRows>
        <KitRow
          :label="$t('settings.launcher.auto_update.label')"
          :description="$t('settings.launcher.auto_update.hint')"
        >
          <Switch v-model="autoUpdate" />
        </KitRow>

        <KitRow
          :label="$t('settings.launcher.telemetry.label')"
          :description="$t('settings.launcher.telemetry.hint')"
        >
          <Switch v-model="telemetry" />
        </KitRow>
      </KitRows>

      <div>
        <h3 class="mb-3 text-title font-semibold text-fg">
          {{ $t('settings.import.title') }}
        </h3>

        <ImportWizard @imported="report => model = report" />
      </div>
    </div>
  </OnboardingPane>
</template>
