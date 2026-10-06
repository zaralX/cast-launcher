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
    :class="cn('flex shrink-0 items-center justify-end gap-3 border-t border-line px-6 py-4', props.class)"
  >
    <slot />
  </div>
</template>
