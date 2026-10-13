<script setup lang="ts">
import type { ContextMenuLabelProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ContextMenuLabel } from 'reka-ui'
import { MENU_LABEL } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<ContextMenuLabelProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <ContextMenuLabel
    data-slot="context-menu-label"
    v-bind="delegatedProps"
    :class="cn(MENU_LABEL, props.class)"
  >
    <slot />
  </ContextMenuLabel>
</template>
