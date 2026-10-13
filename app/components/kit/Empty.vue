<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  icon?: string
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div :class="cn('flex flex-col items-center gap-3 rounded-md border border-dashed border-line-strong px-6 py-12 text-center text-body text-fg-faint', props.class)">
    <Icon
      v-if="icon"
      :name="icon"
      class="size-6"
    />
    <p>
      <slot />
    </p>
    <div
      v-if="$slots.action"
      class="mt-1 flex items-center gap-2"
    >
      <slot name="action" />
    </div>
  </div>
</template>
