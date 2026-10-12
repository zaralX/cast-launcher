<script setup lang="ts">
import type { AppConfig } from '~/types/app'

const config = defineModel<AppConfig>({ required: true })
</script>

<template>
  <div class="space-y-6">
    <KitField :label="$t('settings.launcher.theme')">
      <KitThemePicker v-model="config.launcher.theme" />
    </KitField>

    <KitField
      :label="$t('settings.launcher.accent.label')"
      :hint="$t('settings.launcher.accent.hint')"
    >
      <KitAccentPicker v-model="config.launcher.accent" />
    </KitField>

    <KitRows>
      <KitRow :label="$t('settings.launcher.language')">
        <KitLanguageSelect
          v-model="config.launcher.language"
          class="w-50"
        />
      </KitRow>
      <KitRow
        :label="$t('settings.launcher.compact.label')"
        :description="$t('settings.launcher.compact.hint')"
      >
        <Switch v-model="config.launcher.compact" />
      </KitRow>
    </KitRows>
  </div>
</template>
