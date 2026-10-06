<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  size?: 'md' | 'lg'
  class?: HTMLAttributes['class']
}>(), {
  size: 'md',
})
</script>

<template>
  <div
    data-slot="input-group"
    role="group"
    :class="cn(
      'group/input-group relative flex w-full min-w-0 items-center bg-ink-900 ring ring-inset ring-line transition-colors',
      size === 'lg' ? 'h-9' : 'h-8',
      'has-[[data-slot=input-group-control]:focus-visible]:ring-2 has-[[data-slot=input-group-control]:focus-visible]:ring-acid/60',
      'has-[[data-slot=input-group-control]:disabled]:opacity-75',
      props.class,
    )"
  >
    <slot />
  </div>
</template>
