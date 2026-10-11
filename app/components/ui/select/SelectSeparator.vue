<script setup lang="ts">
import type { SelectSeparatorProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { SelectSeparator } from 'reka-ui'
import { MENU_SEPARATOR } from '@/lib/styles'
import { cn } from '@/lib/utils'
import { Separator } from '@/components/ui/separator'

const props = defineProps<SelectSeparatorProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <SelectSeparator
    v-bind="delegatedProps"
    as-child
  >
    <Separator
      data-slot="select-separator"
      :class="cn(MENU_SEPARATOR, props.class)"
    />
  </SelectSeparator>
</template>
