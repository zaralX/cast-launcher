<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<{
  selected?: boolean
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <button
    type="button"
    :aria-pressed="selected"
    :class="cn(
      FOCUS_RING,
      'group relative cursor-pointer rounded-md bg-panel ring-inset transition-shadow duration-150',
      selected ? 'ring-2 ring-accent' : 'ring-1 ring-card-line hover:ring-line-strong',
      props.class,
    )"
  >
    <slot />
  </button>
</template>
