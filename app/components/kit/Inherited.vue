<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  mono?: boolean
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div :class="cn('rounded-sm bg-sheet px-2.5 py-2 text-caption text-fg-faint', mono && 'font-mono break-all', props.class)">
    <slot />
  </div>
</template>
