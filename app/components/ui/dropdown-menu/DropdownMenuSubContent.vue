<script setup lang="ts">
import type { DropdownMenuSubContentEmits, DropdownMenuSubContentProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { DropdownMenuPortal, DropdownMenuSubContent, useForwardPropsEmits } from 'reka-ui'
import { FLOATING } from '@/lib/styles'
import { cn } from '@/lib/utils'

defineOptions({
  inheritAttrs: false,
})

const props = withDefaults(
  defineProps<DropdownMenuSubContentProps & { class?: HTMLAttributes['class'] }>(),
  {
    sideOffset: 4,
    collisionPadding: 8,
  },
)
const emits = defineEmits<DropdownMenuSubContentEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <DropdownMenuPortal>
    <DropdownMenuSubContent
      data-slot="dropdown-menu-sub-content"
      v-bind="{ ...$attrs, ...forwarded }"
      :class="cn(
        FLOATING,
        'max-h-(--reka-dropdown-menu-content-available-height) min-w-44 origin-(--reka-dropdown-menu-content-transform-origin) overflow-x-hidden overflow-y-auto p-1',
        props.class,
      )"
    >
      <slot />
    </DropdownMenuSubContent>
  </DropdownMenuPortal>
</template>
