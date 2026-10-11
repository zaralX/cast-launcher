<script setup lang="ts">
import type { DropdownMenuCheckboxItemEmits, DropdownMenuCheckboxItemProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { DropdownMenuCheckboxItem, DropdownMenuItemIndicator, useForwardPropsEmits } from 'reka-ui'
import { MENU_INDICATOR, MENU_ITEM } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<DropdownMenuCheckboxItemProps & { class?: HTMLAttributes['class'] }>()
const emits = defineEmits<DropdownMenuCheckboxItemEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <DropdownMenuCheckboxItem
    data-slot="dropdown-menu-checkbox-item"
    v-bind="forwarded"
    :class="cn(MENU_ITEM, 'pr-8', props.class)"
  >
    <slot />
    <DropdownMenuItemIndicator :class="MENU_INDICATOR">
      <Icon
        name="i-lucide-check"
        class="size-4"
      />
    </DropdownMenuItemIndicator>
  </DropdownMenuCheckboxItem>
</template>
