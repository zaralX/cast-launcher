<script setup lang="ts">
import type { AppConfig } from '~/types/app'
import { AFTER_LAUNCH_VALUES } from '~/types/app'

const config = defineModel<AppConfig | null>()

const { t } = useI18n()

const afterLaunchOptions = computed(() => AFTER_LAUNCH_VALUES.map(value => ({
  value,
  label: t(`settings.after_launch.${value}.label`),
  hint: t(`settings.after_launch.${value}.hint`),
})))

const afterLaunchHint = computed(() =>
  afterLaunchOptions.value.find(option => option.value === config.value?.launcher.after_launch)?.hint,
)

async function pickLauncherDir() {
  const picked = await safeRun(() => call('pick_folder', {
    title: t('settings.launcher.dir.label'),
    directory: config.value?.launcher.dir,
  }))

  if (picked) config.value!.launcher.dir = picked
}
</script>

<template>
  <KitPanel
    index="01"
    :title="$t('settings.launcher.title')"
    icon="i-lucide-app-window"
  >
    <div class="space-y-7">
      <div class="grid grid-cols-2 gap-6">
        <KitField :label="$t('settings.launcher.language')">
          <KitLanguageSelect v-model="config!.launcher.language" />
        </KitField>

        <KitField :label="$t('settings.launcher.theme')">
          <KitThemeSelect />
        </KitField>
      </div>

      <KitField
        :label="$t('settings.launcher.accent.label')"
        :hint="$t('settings.launcher.accent.hint')"
      >
        <KitAccentPicker v-model="config!.launcher.accent" />
      </KitField>

      <KitField
        :label="$t('settings.launcher.after_launch.label')"
        :hint="afterLaunchHint"
      >
        <KitSelect
          v-model="config!.launcher.after_launch"
          :items="afterLaunchOptions"
          class="w-1/2"
        />
      </KitField>

      <KitField
        :label="$t('settings.launcher.dir.label')"
        :hint="$t('settings.launcher.dir.hint')"
      >
        <div class="flex gap-2">
          <Input
            v-model="config!.launcher.dir"
            placeholder="/path/to/launcher"
            size="lg"
            font="mono"
          />

          <Button
            icon="i-lucide-folder-open"
            @click="pickLauncherDir"
          >
            {{ $t('common.pick') }}
          </Button>
        </div>
      </KitField>

      <KitRow
        :label="$t('settings.launcher.compact.label')"
        :description="$t('settings.launcher.compact.hint')"
      >
        <Switch
          v-model="config!.launcher.compact"
          size="lg"
        />
      </KitRow>

      <KitRow
        :label="$t('settings.launcher.auto_update.label')"
        :description="$t('settings.launcher.auto_update.hint')"
      >
        <Switch
          v-model="config!.launcher.auto_update"
          size="lg"
        />
      </KitRow>

      <KitRow
        :label="$t('settings.launcher.telemetry.label')"
        :description="$t('settings.launcher.telemetry.hint')"
      >
        <Switch
          v-model="config!.launcher.telemetry"
          size="lg"
        />
      </KitRow>
    </div>
  </KitPanel>
</template>
