<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div
    data-slot="dialog-header"
    :class="cn('flex min-h-16 shrink-0 flex-col justify-center gap-1 border-b border-line py-4 pr-16 pl-6', props.class)"
  >
    <slot />
  </div>
</template>
