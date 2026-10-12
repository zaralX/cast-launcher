<script setup lang="ts">
import type { AppConfig } from '~/types/app'
import { AFTER_LAUNCH_VALUES } from '~/types/app'

const config = defineModel<AppConfig>({ required: true })

const { t } = useI18n()

const afterLaunchOptions = computed(() => AFTER_LAUNCH_VALUES.map(value => ({
  value,
  label: t(`settings.after_launch.${value}.label`),
  hint: value === 'nothing' ? undefined : t(`settings.after_launch.${value}.hint`),
})))

const afterLaunchHint = computed(() =>
  afterLaunchOptions.value.find(option => option.value === config.value.launcher.after_launch)?.hint,
)
</script>

<template>
  <KitRows>
    <KitRow
      :label="$t('settings.launcher.after_launch.label')"
      :description="afterLaunchHint"
    >
      <KitSelect
        v-model="config.launcher.after_launch"
        :items="afterLaunchOptions"
        class="w-56"
      />
    </KitRow>
    <KitRow
      :label="$t('settings.launcher.auto_update.label')"
    >
      <Switch v-model="config.launcher.auto_update" />
    </KitRow>
    <KitRow
      :label="$t('settings.launcher.telemetry.label')"
      :description="$t('settings.launcher.telemetry.hint')"
    >
      <Switch v-model="config.launcher.telemetry" />
    </KitRow>
  </KitRows>
</template>
