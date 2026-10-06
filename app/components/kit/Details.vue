<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  columns?: 1 | 2
  class?: HTMLAttributes['class']
}>(), {
  columns: 2,
  class: undefined,
})
</script>

<template>
  <dl :class="cn('grid gap-x-8 gap-y-3', columns === 2 && 'grid-cols-2', props.class)">
    <slot />
  </dl>
</template>
