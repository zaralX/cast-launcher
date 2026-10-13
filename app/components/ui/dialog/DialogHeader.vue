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
    :class="cn('flex shrink-0 flex-col gap-1.5 pt-5 pr-14 pl-5', props.class)"
  >
    <slot />
  </div>
</template>
