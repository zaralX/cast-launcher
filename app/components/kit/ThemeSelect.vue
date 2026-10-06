<script setup lang="ts">
import type { SelectOption } from '~/types/ui'

const colorMode = useColorMode()
const { t } = useI18n()

const options = computed<SelectOption[]>(() => [
  { value: 'system', label: t('settings.launcher.theme.system'), icon: 'i-lucide-monitor' },
  { value: 'light', label: t('settings.launcher.theme.light'), icon: 'i-lucide-sun' },
  { value: 'dark', label: t('settings.launcher.theme.dark'), icon: 'i-lucide-moon' },
])
</script>

<template>
  <KitSelect
    v-model="colorMode.preference"
    :items="options"
  />
</template>
