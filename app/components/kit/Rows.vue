<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div :class="cn('divide-y divide-line rounded-md bg-panel px-4 ring-1 ring-card-line ring-inset *:py-3', props.class)">
    <slot />
  </div>
</template>
