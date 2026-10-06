<script setup lang="ts">
import type { StatusTone } from '~/types/ui'

withDefaults(defineProps<{
  icon: string
  tone?: StatusTone
}>(), {
  tone: 'warning',
})

const TONE: Record<StatusTone, string> = {
  muted: 'text-fg-faint',
  accent: 'text-acid',
  warning: 'text-warning',
  danger: 'text-danger',
}
</script>

<template>
  <p class="flex items-start gap-2.5 text-body leading-relaxed text-fg-muted">
    <Icon
      :name="icon"
      class="mt-0.5 size-3.5 shrink-0"
      :class="TONE[tone]"
    />
    <span class="min-w-0">
      <slot />
    </span>
  </p>
</template>
