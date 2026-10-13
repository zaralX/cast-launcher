<script setup lang="ts">
import type { SelectItemProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  SelectItem,
  SelectItemIndicator,
  SelectItemText,
  useForwardProps,
} from 'reka-ui'
import { MENU_INDICATOR, MENU_ITEM } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<SelectItemProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <SelectItem
    data-slot="select-item"
    v-bind="forwardedProps"
    :class="cn(MENU_ITEM, 'pr-8', props.class)"
  >
    <SelectItemText class="flex min-w-0 items-center gap-2.5 truncate">
      <slot />
    </SelectItemText>

    <SelectItemIndicator :class="MENU_INDICATOR">
      <Icon
        name="i-lucide-check"
        class="size-4"
      />
    </SelectItemIndicator>
  </SelectItem>
</template>
