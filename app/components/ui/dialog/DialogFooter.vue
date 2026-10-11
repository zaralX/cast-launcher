<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div
    data-slot="dialog-footer"
    :class="cn('flex shrink-0 items-center justify-end gap-2 p-5', props.class)"
  >
    <slot />
  </div>
</template>
