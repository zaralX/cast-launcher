<script setup lang="ts">
import type { ContextMenuItemEmits, ContextMenuItemProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  ContextMenuItem,
  useForwardPropsEmits,
} from 'reka-ui'
import { MENU_ICON, MENU_ITEM } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<ContextMenuItemProps & {
  class?: HTMLAttributes['class']
  icon?: string
  variant?: 'default' | 'danger'
}>(), {
  class: undefined,
  icon: undefined,
  variant: 'default',
})
const emits = defineEmits<ContextMenuItemEmits>()

const delegatedProps = reactiveOmit(props, 'class', 'icon', 'variant')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <ContextMenuItem
    data-slot="context-menu-item"
    :data-variant="variant"
    v-bind="forwarded"
    :class="cn(MENU_ITEM, props.class)"
  >
    <Icon
      v-if="icon"
      :name="icon"
      :class="MENU_ICON"
    />
    <slot />
  </ContextMenuItem>
</template>
