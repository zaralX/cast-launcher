<script setup lang="ts">
import type { DropdownMenuSubTriggerProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { DropdownMenuSubTrigger, useForwardProps } from 'reka-ui'
import { MENU_ICON, MENU_ITEM } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<DropdownMenuSubTriggerProps & {
  class?: HTMLAttributes['class']
  icon?: string
}>()

const delegatedProps = reactiveOmit(props, 'class', 'icon')
const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <DropdownMenuSubTrigger
    data-slot="dropdown-menu-sub-trigger"
    v-bind="forwardedProps"
    :class="cn(MENU_ITEM, props.class)"
  >
    <Icon
      v-if="icon"
      :name="icon"
      :class="MENU_ICON"
    />
    <slot />
    <Icon
      name="i-lucide-chevron-right"
      class="ml-auto size-4 text-fg-faint"
    />
  </DropdownMenuSubTrigger>
</template>
