<script setup lang="ts">
import type { ContextMenuSeparatorProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ContextMenuSeparator } from 'reka-ui'
import { MENU_SEPARATOR } from '@/lib/styles'
import { cn } from '@/lib/utils'
import { Separator } from '@/components/ui/separator'

const props = defineProps<ContextMenuSeparatorProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <ContextMenuSeparator
    v-bind="delegatedProps"
    as-child
  >
    <Separator
      data-slot="context-menu-separator"
      :class="cn(MENU_SEPARATOR, props.class)"
    />
  </ContextMenuSeparator>
</template>
