<script setup lang="ts">
import type { SelectTriggerProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { SelectIcon, SelectTrigger, useForwardProps } from 'reka-ui'
import { FIELD } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<SelectTriggerProps & { class?: HTMLAttributes['class'], size?: 'sm' | 'md' | 'lg' }>(),
  { size: 'md' },
)

const SIZE = {
  sm: 'h-7 px-2.5 text-caption',
  md: 'h-8 px-2.5',
  lg: 'h-10 px-3 text-title',
}

const delegatedProps = reactiveOmit(props, 'class', 'size')
const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <SelectTrigger
    data-slot="select-trigger"
    :data-size="size"
    v-bind="forwardedProps"
    :class="cn(
      FIELD,
      'flex w-full cursor-pointer items-center justify-between gap-2 text-left whitespace-nowrap hover:bg-hover data-[placeholder]:text-fg-faint data-[state=open]:shadow-focus',
      '*:data-[slot=select-value]:flex *:data-[slot=select-value]:min-w-0 *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-2 *:data-[slot=select-value]:truncate',
      SIZE[size],
      props.class,
    )"
  >
    <slot />
    <SelectIcon as-child>
      <Icon
        name="i-lucide-chevron-down"
        class="size-4 shrink-0 text-fg-faint"
      />
    </SelectIcon>
  </SelectTrigger>
</template>
