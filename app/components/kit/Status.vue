<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { StatusTone } from '~/types/ui'
import { STATUS_TONE } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  tone?: StatusTone
  dot?: 'static' | 'live' | 'none'
  class?: HTMLAttributes['class']
}>(), {
  tone: 'muted',
  dot: 'static',
  class: undefined,
})
</script>

<template>
  <p :class="cn('flex items-start gap-2 text-caption font-medium', STATUS_TONE[tone], props.class)">
    <span
      v-if="dot !== 'none'"
      class="flex h-[1.4em] shrink-0 items-center"
    >
      <KitLiveDot
        v-if="dot === 'live'"
        size="sm"
      />
      <span
        v-else
        class="size-1.5 rounded-pill bg-current"
      />
    </span>
    <span class="min-w-0">
      <slot />
    </span>
  </p>
</template>
