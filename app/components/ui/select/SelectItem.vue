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
import { MENU_ITEM } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<SelectItemProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <SelectItem
    data-slot="select-item"
    v-bind="forwardedProps"
    :class="cn(MENU_ITEM, 'pr-8 text-lead', props.class)"
  >
    <SelectItemText class="truncate">
      <slot />
    </SelectItemText>

    <SelectItemIndicator class="absolute right-1.5 flex items-center">
      <Icon
        name="i-lucide-check"
        class="size-4 text-acid"
      />
    </SelectItemIndicator>
  </SelectItem>
</template>
