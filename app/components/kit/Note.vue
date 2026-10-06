<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { StatusTone } from '~/types/ui'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  icon: string
  tone?: StatusTone
  class?: HTMLAttributes['class']
}>(), {
  tone: 'warning',
  class: undefined,
})

const TONE: Record<StatusTone, string> = {
  muted: 'text-fg-faint',
  accent: 'text-acid',
  warning: 'text-warning',
  danger: 'text-danger',
}
</script>

<template>
  <p :class="cn('flex items-start gap-2.5 text-body leading-relaxed text-fg-muted', props.class)">
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
