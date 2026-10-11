<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  label?: string
  description?: string
  tone?: 'default' | 'danger'
  class?: HTMLAttributes['class']
}>(), {
  label: undefined,
  description: undefined,
  tone: 'default',
  class: undefined,
})
</script>

<template>
  <div :class="cn('flex items-center gap-6', props.class)">
    <div class="min-w-0 flex-1">
      <p
        v-if="label"
        class="font-medium"
        :class="tone === 'danger' ? 'text-danger' : 'text-fg'"
      >
        {{ label }}
      </p>
      <div
        v-if="description || $slots.description"
        class="mt-0.5 text-caption text-fg-faint"
      >
        <slot name="description">
          {{ description }}
        </slot>
      </div>
    </div>

    <div class="flex shrink-0 items-center gap-2">
      <slot />
    </div>
  </div>
</template>
