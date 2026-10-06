<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <p
    data-slot="alert-title"
    :class="cn('text-title font-medium text-fg', props.class)"
  >
    <slot />
  </p>
</template>
