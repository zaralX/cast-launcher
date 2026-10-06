<script setup lang="ts">
import type { PopoverContentEmits, PopoverContentProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  PopoverContent,
  PopoverPortal,
  useForwardPropsEmits,
} from 'reka-ui'
import { FLOATING } from '@/lib/styles'
import { cn } from '@/lib/utils'

defineOptions({
  inheritAttrs: false,
})

const props = withDefaults(
  defineProps<PopoverContentProps & { class?: HTMLAttributes['class'] }>(),
  {
    align: 'center',
    sideOffset: 8,
    collisionPadding: 8,
  },
)
const emits = defineEmits<PopoverContentEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <PopoverPortal>
    <PopoverContent
      data-slot="popover-content"
      v-bind="{ ...$attrs, ...forwarded }"
      :class="
        cn(FLOATING, 'origin-(--reka-popover-content-transform-origin)', props.class)
      "
    >
      <slot />
    </PopoverContent>
  </PopoverPortal>
</template>
