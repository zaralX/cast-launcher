<script setup lang="ts">
import type { AppConfig } from '~/types/app'

const config = defineModel<AppConfig>({ required: true })
</script>

<template>
  <KitRows>
    <KitRow
      :label="$t('settings.discord.enabled.label')"
      :description="$t('settings.discord.enabled.hint')"
    >
      <Switch v-model="config.discord.enabled" />
    </KitRow>
    <KitRow
      :label="$t('settings.discord.instance.label')"
      :description="$t('settings.discord.instance.hint')"
    >
      <Switch
        v-model="config.discord.show_instance"
        :disabled="!config.discord.enabled"
      />
    </KitRow>
  </KitRows>
</template>
