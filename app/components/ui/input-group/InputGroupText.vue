<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <span :class="cn('flex items-center gap-2 font-mono text-label uppercase tracking-caps text-fg-faint', props.class)">
    <slot />
  </span>
</template>
