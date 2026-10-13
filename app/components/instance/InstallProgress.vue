<script setup lang="ts">
import type { UiText } from '~/types/backend'

defineProps<{
  phase?: UiText | null
  progress?: number | null
}>()
</script>

<template>
  <div class="flex flex-col justify-center gap-1.5">
    <span class="flex items-baseline justify-between gap-3 text-caption">
      <span class="min-w-0 truncate text-fg-muted">
        {{ phase ? uiText(phase) : $t('instance.state.installing') }}
      </span>
      <span
        v-if="progress != null"
        class="shrink-0 text-fg tabular-nums"
      >
        {{ Math.round(progress * 100) }}%
      </span>
    </span>

    <Progress :model-value="progress == null ? null : progress * 100" />
  </div>
</template>
