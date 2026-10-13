<script setup lang="ts">
defineProps<{
  label?: string
  disabled?: boolean
}>()

const model = defineModel<boolean>({ default: false })
</script>

<template>
  <label class="flex cursor-pointer items-center gap-2 text-caption text-fg-faint">
    {{ label ?? $t('common.own') }}
    <Switch
      v-model="model"
      :disabled="disabled"
    />
  </label>
</template>
