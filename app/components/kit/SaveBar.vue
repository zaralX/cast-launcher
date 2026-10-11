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
  <Transition
    enter-active-class="transition duration-200 ease-deck"
    leave-active-class="transition duration-150"
    enter-from-class="translate-y-3 opacity-0"
    leave-to-class="translate-y-3 opacity-0"
  >
    <div
      v-if="dirty || saving"
      role="status"
      class="fixed right-7 bottom-5 z-40 flex items-center gap-2.5 rounded-md bg-raise py-2 pr-2 pl-4 shadow-float ring-1 ring-line-strong"
    >
      <span class="text-body text-fg-muted">{{ dirtyLabel }}</span>
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
    </div>
  </Transition>
</template>
