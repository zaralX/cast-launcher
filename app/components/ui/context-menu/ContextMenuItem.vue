<script setup lang="ts">
import type { ContextMenuItemEmits, ContextMenuItemProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  ContextMenuItem,
  useForwardPropsEmits,
} from 'reka-ui'
import { MENU_ITEM } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<ContextMenuItemProps & {
  class?: HTMLAttributes['class']
  inset?: boolean
  variant?: 'default' | 'danger'
}>(), {
  variant: 'default',
})
const emits = defineEmits<ContextMenuItemEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <ContextMenuItem
    data-slot="context-menu-item"
    :data-inset="inset ? '' : undefined"
    :data-variant="variant"
    v-bind="forwarded"
    :class="cn(MENU_ITEM, 'font-mono text-caption data-[variant=danger]:text-danger data-[variant=danger]:data-[highlighted]:text-danger', props.class)"
  >
    <slot />
  </ContextMenuItem>
</template>
