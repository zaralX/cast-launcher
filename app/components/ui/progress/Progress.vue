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
const props = defineProps<ProgressRootProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')

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
      :class="percent == null
        ? 'absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep'
        : 'absolute inset-y-0 left-0 bg-acid transition-[width] duration-500 ease-deck'"
      :style="percent == null ? undefined : { width: `${percent}%` }"
    />
  </ProgressRoot>
</template>
