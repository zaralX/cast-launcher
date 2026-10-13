<script setup lang="ts">
import type { ComboboxGroupProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ComboboxGroup, ComboboxLabel } from 'reka-ui'
import { MENU_LABEL } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<ComboboxGroupProps & {
  class?: HTMLAttributes['class']
  heading?: string
}>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <ComboboxGroup
    data-slot="combobox-group"
    v-bind="delegatedProps"
    :class="cn('overflow-hidden p-1', props.class)"
  >
    <ComboboxLabel
      v-if="heading"
      :class="MENU_LABEL"
    >
      {{ heading }}
    </ComboboxLabel>
    <slot />
  </ComboboxGroup>
</template>
