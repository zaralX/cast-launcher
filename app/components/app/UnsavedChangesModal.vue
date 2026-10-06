<script setup lang="ts">
import type { UnsavedChanges } from '~/composables/useUnsavedChanges'

const props = defineProps<{
  guard: UnsavedChanges
  description?: string
  blocked?: string
  discardLabel?: string
}>()

const { t } = useI18n()

const descriptionText = computed(() => props.description ?? t('unsaved.description'))
const discardText = computed(() => props.discardLabel ?? t('unsaved.discard'))
</script>

<template>
  <Dialog :open="guard.open">
    <DialogContent
      :dismissible="false"
      :show-close-button="false"
    >
      <DialogHeader>
        <DialogTitle>{{ $t('unsaved.title') }}</DialogTitle>
      </DialogHeader>

      <DialogBody class="space-y-6">
        <DialogDescription>
          {{ descriptionText }}
        </DialogDescription>

        <KitStatus
          v-if="blocked && !guard.canSave"
          tone="warning"
        >
          {{ blocked }}
        </KitStatus>
      </DialogBody>

      <DialogFooter class="justify-between">
        <Button
          variant="quiet-danger"
          icon="i-lucide-trash-2"
          :disabled="guard.saving"
          @click="guard.discard()"
        >
          {{ discardText }}
        </Button>

        <div class="flex items-center gap-4">
          <Button
            variant="quiet"
            :disabled="guard.saving"
            @click="guard.cancel()"
          >
            {{ $t('unsaved.stay') }}
          </Button>

          <Button
            size="lg"
            icon="i-lucide-save"
            :loading="guard.saving"
            :disabled="!guard.canSave"
            @click="guard.save()"
          >
            {{ guard.saving ? $t('common.saving') : $t('common.save') }}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
