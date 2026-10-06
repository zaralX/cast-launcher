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
  <div>
    <Button
      size="xl"
      icon="i-lucide-save"
      class="w-full"
      :loading="saving"
      :disabled="disabled"
      @click="emit('save')"
    >
      {{ saving ? $t('common.saving') : $t('common.save') }}
    </Button>

    <slot />

    <div
      v-if="dirty"
      class="mt-4 flex items-center justify-between gap-3"
    >
      <KitStatus tone="warning">
        {{ dirtyLabel }}
      </KitStatus>

      <Button
        variant="quiet"
        @click="emit('reset')"
      >
        {{ $t('common.reset') }}
      </Button>
    </div>
  </div>
</template>
