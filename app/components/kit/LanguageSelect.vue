<script setup lang="ts">
import type { SelectOption } from '~/types/ui'

const model = defineModel<string>()

const locales = useAvailableLocales()

const options = computed<SelectOption[]>(() => locales.value.map(locale => ({
  value: locale.code,
  label: locale.name,
  icon: flagOf(locale.code) ?? undefined,
})))
</script>

<template>
  <KitSelect
    v-model="model"
    :items="options"
  />
</template>
