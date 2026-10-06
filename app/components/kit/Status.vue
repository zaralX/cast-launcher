<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { StatusTone } from '~/types/ui'
import { STATUS_TONE } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  tone?: StatusTone
  dot?: 'blink' | 'static' | 'live'
  class?: HTMLAttributes['class']
}>(), {
  tone: 'muted',
  dot: 'blink',
  class: undefined,
})
</script>

<template>
  <p :class="cn('flex items-start gap-2.5 font-mono text-label uppercase leading-relaxed tracking-caps', STATUS_TONE[tone], props.class)">
    <span class="flex h-[1.625em] shrink-0 items-center">
      <KitLiveDot
        v-if="dot === 'live'"
        size="sm"
      />
      <span
        v-else
        class="size-1.5 bg-current"
        :class="dot === 'blink' && 'animate-blink'"
      />
    </span>
    <span class="min-w-0">
      <slot />
    </span>
  </p>
</template>
