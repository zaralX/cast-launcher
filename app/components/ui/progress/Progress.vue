<script setup lang="ts">
import type { ProgressRootProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  ProgressIndicator,
  ProgressRoot,
} from 'reka-ui'
import { cn } from '@/lib/utils'

// A null modelValue is an indeterminate bar.
const props = withDefaults(defineProps<ProgressRootProps & {
  tone?: 'accent' | 'muted'
  class?: HTMLAttributes['class']
}>(), {
  tone: 'accent',
  class: undefined,
})

const delegatedProps = reactiveOmit(props, 'class', 'tone')

const percent = computed(() => {
  if (props.modelValue == null) return null
  const max = props.max ?? 100
  return Math.min(Math.max(props.modelValue / max, 0), 1) * 100
})
</script>

<template>
  <ProgressRoot
    data-slot="progress"
    v-bind="delegatedProps"
    :class="cn('relative block h-px w-full overflow-hidden bg-line', props.class)"
  >
    <ProgressIndicator
      data-slot="progress-indicator"
      :class="cn(
        'absolute inset-y-0 left-0',
        tone === 'muted' ? 'bg-fg-muted' : 'bg-acid',
        percent == null ? 'w-1/4 animate-sweep' : 'transition-[width] duration-500 ease-deck',
      )"
      :style="percent == null ? undefined : { width: `${percent}%` }"
    />
  </ProgressRoot>
</template>
