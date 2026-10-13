<script setup lang="ts">
import type { ComboboxItemIndicatorProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ComboboxItemIndicator, useForwardProps } from 'reka-ui'
import { MENU_INDICATOR } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<ComboboxItemIndicatorProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardProps(delegatedProps)
</script>

<template>
  <ComboboxItemIndicator
    data-slot="combobox-item-indicator"
    v-bind="forwarded"
    :class="cn(MENU_INDICATOR, props.class)"
  >
    <slot />
  </ComboboxItemIndicator>
</template>
