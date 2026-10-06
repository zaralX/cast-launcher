<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
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
      'group relative cursor-pointer border outline-none transition-colors duration-300 focus-visible:border-line-strong',
      selected ? 'border-acid bg-ink-700' : 'border-line hover:border-line-strong hover:bg-ink-700',
      props.class,
    )"
  >
    <slot />
  </button>
</template>
