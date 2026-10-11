<script setup lang="ts">
import type { DropdownMenuSeparatorProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { DropdownMenuSeparator } from 'reka-ui'
import { MENU_SEPARATOR } from '@/lib/styles'
import { cn } from '@/lib/utils'
import { Separator } from '@/components/ui/separator'

const props = defineProps<DropdownMenuSeparatorProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <DropdownMenuSeparator
    v-bind="delegatedProps"
    as-child
  >
    <Separator
      data-slot="dropdown-menu-separator"
      :class="cn(MENU_SEPARATOR, props.class)"
    />
  </DropdownMenuSeparator>
</template>
