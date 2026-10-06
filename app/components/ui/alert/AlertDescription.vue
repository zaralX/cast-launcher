<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div
    data-slot="alert-description"
    :class="cn('text-body leading-relaxed text-fg-muted not-first:mt-2', props.class)"
  >
    <slot />
  </div>
</template>
