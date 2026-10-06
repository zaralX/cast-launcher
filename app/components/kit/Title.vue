<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  as?: string
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'
  class?: HTMLAttributes['class']
}>(), {
  as: 'h1',
  size: 'lg',
  class: undefined,
})

const SIZE = {
  xs: 'text-lg leading-none xl:text-xl',
  sm: 'text-[clamp(18px,2vw,24px)] leading-none',
  md: 'text-[clamp(22px,2.6vw,30px)] leading-[0.95]',
  lg: 'text-[clamp(26px,3vw,34px)] leading-[0.95]',
  xl: 'text-[clamp(30px,6vw,52px)] leading-[0.9]',
}
</script>

<template>
  <component
    :is="as"
    :class="cn('font-unbounded font-bold tracking-display text-fg', SIZE[size], props.class)"
  >
    <slot /><span class="text-acid">.</span>
  </component>
</template>
