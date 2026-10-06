<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { StatusTone } from '~/types/ui'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  tone?: StatusTone
  blink?: boolean
  class?: HTMLAttributes['class']
}>(), {
  tone: 'muted',
  blink: true,
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
  <p :class="cn('flex items-start gap-2.5 font-mono text-label uppercase leading-relaxed tracking-caps', TONE[tone], props.class)">
    <span
      class="mt-[0.5em] size-1.5 shrink-0 bg-current"
      :class="blink && 'animate-blink'"
    />
    <span class="min-w-0">
      <slot />
    </span>
  </p>
</template>
