<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <p :class="cn('border border-dashed border-line py-12 text-center font-mono text-label uppercase tracking-caps text-fg-faint', props.class)">
    <slot />
  </p>
</template>
