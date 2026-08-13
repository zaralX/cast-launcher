<script setup lang="ts">
import type {UnsavedChanges} from "~/composables/useUnsavedChanges"

withDefaults(defineProps<{
  guard: UnsavedChanges
  description?: string
  blocked?: string
  discardLabel?: string
}>(), {
  description: "Изменения на этой странице ещё не сохранены. Если уйти сейчас - они пропадут.",
  discardLabel: "Не сохранять"
})
</script>

<template>
  <UModal
      :open="guard.open"
      :dismissible="false"
      :close="false"
      title="Несохранённые изменения"
      :ui="{ content: 'max-w-lg' }"
  >
    <template #body>
      <div class="space-y-6">
        <p class="text-[12px] leading-relaxed text-fg-muted">{{ description }}</p>

        <p
            v-if="blocked && !guard.canSave"
            class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-amber-400"
        >
          <span class="size-1.5 bg-amber-400 animate-blink"/>
          {{ blocked }}
        </p>

        <div class="flex items-center justify-between gap-4 border-t border-line pt-5">
          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.16em] text-fg-faint hover:text-red-400"
              icon="i-lucide-trash-2"
              :disabled="guard.saving"
              @click="guard.discard()"
          >
            {{ discardLabel }}
          </AppButton>

          <div class="flex items-center gap-4">
            <AppButton
                tone="quiet"
                class="text-[10px] tracking-[0.16em]"
                :disabled="guard.saving"
                @click="guard.cancel()"
            >
              Остаться
            </AppButton>

            <AppButton
                class="h-10 px-6 tracking-[0.18em]"
                icon="i-lucide-save"
                :loading="guard.saving"
                :disabled="!guard.canSave"
                @click="guard.save()"
            >
              {{ guard.saving ? 'Сохранение' : 'Сохранить' }}
            </AppButton>
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>
