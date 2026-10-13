<script setup lang="ts">
import type { DropdownMenuRadioItemEmits, DropdownMenuRadioItemProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { DropdownMenuItemIndicator, DropdownMenuRadioItem, useForwardPropsEmits } from 'reka-ui'
import { MENU_INDICATOR, MENU_ITEM } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<DropdownMenuRadioItemProps & { class?: HTMLAttributes['class'] }>()
const emits = defineEmits<DropdownMenuRadioItemEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <DropdownMenuRadioItem
    data-slot="dropdown-menu-radio-item"
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
  </DropdownMenuRadioItem>
</template>
