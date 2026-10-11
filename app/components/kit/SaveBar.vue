<script setup lang="ts">
defineProps<{
  saving?: boolean
  disabled?: boolean
  dirty?: boolean
  dirtyLabel: string
}>()

const emit = defineEmits<{
  save: []
  reset: []
}>()
</script>

<template>
  <KitFloatingBar
    :open="!!(dirty || saving)"
    :label="dirtyLabel"
  >
    <Button
      variant="ghost"
      size="sm"
      :disabled="saving"
      @click="emit('reset')"
    >
      {{ $t('common.reset') }}
    </Button>
    <Button
      variant="strong"
      size="sm"
      icon="i-lucide-save"
      :loading="saving"
      :disabled="disabled"
      @click="emit('save')"
    >
      {{ saving ? $t('common.saving') : $t('common.save') }}
    </Button>
  </KitFloatingBar>
</template>
