<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  selected?: boolean
  variant?: 'card' | 'ghost'
  class?: HTMLAttributes['class']
}>(), {
  variant: 'card',
  class: undefined,
})

const VARIANT = {
  card: {
    base: 'bg-panel ring-inset transition-shadow',
    idle: 'ring-1 ring-card-line hover:ring-line-strong',
    selected: 'ring-2 ring-accent',
  },
  ghost: {
    base: 'transition-colors',
    idle: 'hover:bg-panel',
    selected: 'bg-accent/18',
  },
}
</script>

<template>
  <button
    type="button"
    :aria-pressed="selected"
    :class="cn(
      FOCUS_RING,
      'group relative cursor-pointer rounded-md duration-150',
      VARIANT[variant].base,
      selected ? VARIANT[variant].selected : VARIANT[variant].idle,
      props.class,
    )"
  >
    <slot />
  </button>
</template>
