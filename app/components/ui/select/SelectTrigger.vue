<script setup lang="ts">
import type { SelectTriggerProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { SelectIcon, SelectTrigger, useForwardProps } from 'reka-ui'
import { FIELD } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<SelectTriggerProps & { class?: HTMLAttributes['class'], size?: 'md' | 'lg' }>(),
  { size: 'md' },
)

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
      'flex w-full cursor-pointer items-center justify-between gap-1.5 text-left whitespace-nowrap data-[placeholder]:text-fg-faint',
      '*:data-[slot=select-value]:truncate',
      size === 'lg' ? 'h-9 pr-2.5 pl-3' : 'h-8 pr-2 pl-2.5',
      props.class,
    )"
  >
    <slot />
    <SelectIcon as-child>
      <Icon
        name="i-lucide-chevron-down"
        class="size-5 shrink-0 text-fg-faint"
      />
    </SelectIcon>
  </SelectTrigger>
</template>
