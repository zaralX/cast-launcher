<script setup lang="ts">
withDefaults(defineProps<{
  title: string
  description?: string
  confirmLabel: string
  confirmIcon?: string
  cancelLabel?: string
  tone?: 'danger' | 'default'
  loading?: boolean
}>(), {
  description: undefined,
  confirmIcon: 'i-lucide-trash-2',
  cancelLabel: undefined,
  tone: 'danger',
  loading: false,
})

const emit = defineEmits<{ confirm: [] }>()

const open = defineModel<boolean>('open', { default: false })
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      :dismissible="!loading"
      :show-close-button="!loading"
    >
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
      </DialogHeader>

      <DialogBody class="space-y-5">
        <DialogDescription v-if="description">
          {{ description }}
        </DialogDescription>
        <slot />
      </DialogBody>

      <DialogFooter>
        <Button
          variant="quiet"
          :disabled="loading"
          @click="open = false"
        >
          {{ cancelLabel ?? $t('common.cancel') }}
        </Button>
        <Button
          :variant="tone === 'danger' ? 'danger' : 'fill'"
          size="sm"
          :icon="confirmIcon"
          :loading="loading"
          @click="emit('confirm')"
        >
          {{ confirmLabel }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
