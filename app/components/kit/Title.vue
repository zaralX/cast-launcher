<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  as?: string
  size?: 'sm' | 'md' | 'lg'
  class?: HTMLAttributes['class']
}>(), {
  as: 'h1',
  size: 'md',
  class: undefined,
})

const SIZE = {
  sm: 'text-heading',
  md: 'text-page',
  lg: 'text-display',
}
</script>

<template>
  <component
    :is="as"
    :class="cn('font-unbounded font-semibold tracking-heading text-fg classic:uppercase classic:tracking-normal', SIZE[size], props.class)"
  >
    <slot />
  </component>
</template>
