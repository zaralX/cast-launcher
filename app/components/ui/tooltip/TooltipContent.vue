<script setup lang="ts">
import type { TooltipContentEmits, TooltipContentProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { TooltipContent, TooltipPortal, useForwardPropsEmits } from 'reka-ui'
import { cn } from '@/lib/utils'

defineOptions({
  inheritAttrs: false,
})

const props = withDefaults(defineProps<TooltipContentProps & { class?: HTMLAttributes['class'] }>(), {
  sideOffset: 10,
})

const emits = defineEmits<TooltipContentEmits>()

const delegatedProps = reactiveOmit(props, 'class')
const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <TooltipPortal>
    <TooltipContent
      data-slot="tooltip-content"
      v-bind="{ ...forwarded, ...$attrs }"
      :class="cn(
        'z-50 w-fit border border-line bg-ink-700 px-2.5 py-1 font-mono text-label whitespace-nowrap uppercase tracking-caps text-fg duration-300 ease-deck',
        'animate-in fade-in-0 data-[side=right]:slide-in-from-left-2 data-[side=left]:slide-in-from-right-2 data-[side=top]:slide-in-from-bottom-1 data-[side=bottom]:slide-in-from-top-1',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
        props.class,
      )"
    >
      <slot />
    </TooltipContent>
  </TooltipPortal>
</template>
