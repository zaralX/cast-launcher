<script setup lang="ts">
import type { SelectOption } from '~/types/ui'

const colorMode = useColorMode()
const { t } = useI18n()

const options = computed<SelectOption[]>(() => [
  { value: 'system', label: t('settings.launcher.theme.system'), icon: 'i-lucide-monitor' },
  { value: 'light', label: t('settings.launcher.theme.light'), icon: 'i-lucide-sun' },
  { value: 'dark', label: t('settings.launcher.theme.dark'), icon: 'i-lucide-moon' },
])

const preference = computed({
  get: () => colorMode.preference,
  set: (value: string) => {
    colorMode.preference = value
  },
})
</script>

<template>
  <KitSelect
    v-model="preference"
    :items="options"
  />
</template>
