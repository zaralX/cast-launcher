<script setup lang="ts">
import type { ComboboxSeparatorProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ComboboxSeparator } from 'reka-ui'
import { MENU_SEPARATOR } from '@/lib/styles'
import { cn } from '@/lib/utils'
import { Separator } from '@/components/ui/separator'

const props = defineProps<ComboboxSeparatorProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <ComboboxSeparator
    v-bind="delegatedProps"
    as-child
  >
    <Separator
      data-slot="combobox-separator"
      :class="cn(MENU_SEPARATOR, props.class)"
    />
  </ComboboxSeparator>
</template>
