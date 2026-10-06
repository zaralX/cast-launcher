<script setup lang="ts">
import type { SwitchRootEmits, SwitchRootProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  SwitchRoot,
  SwitchThumb,
  useForwardPropsEmits,
} from 'reka-ui'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<SwitchRootProps & {
  size?: 'md' | 'lg'
  class?: HTMLAttributes['class']
}>(), {
  size: 'md',
})

const emits = defineEmits<SwitchRootEmits>()

const delegatedProps = reactiveOmit(props, 'class', 'size')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <SwitchRoot
    v-slot="slotProps"
    data-slot="switch"
    v-bind="forwarded"
    :class="cn(
      'peer inline-flex shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent outline-none transition-[background] duration-200',
      'data-[state=checked]:bg-acid data-[state=unchecked]:bg-line-strong',
      'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acid disabled:cursor-not-allowed disabled:opacity-75',
      size === 'lg' ? 'h-6 w-11' : 'h-5 w-9',
      props.class,
    )"
  >
    <SwitchThumb
      data-slot="switch-thumb"
      :class="cn(
        'pointer-events-none block rounded-full bg-ink-800 shadow-lg ring-0 transition-transform duration-200 data-[state=unchecked]:translate-x-0',
        size === 'lg' ? 'size-5 data-[state=checked]:translate-x-5' : 'size-4 data-[state=checked]:translate-x-4',
      )"
    >
      <slot
        name="thumb"
        v-bind="slotProps"
      />
    </SwitchThumb>
  </SwitchRoot>
</template>
