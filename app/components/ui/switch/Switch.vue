<script setup lang="ts">
import type { SwitchRootEmits, SwitchRootProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  SwitchRoot,
  SwitchThumb,
  useForwardPropsEmits,
} from 'reka-ui'
import { FOCUS_RING } from '@/lib/styles'
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
      FOCUS_RING,
      'peer inline-flex shrink-0 cursor-pointer items-center rounded-pill p-0.75 transition-colors duration-150',
      'data-[state=checked]:bg-accent data-[state=unchecked]:bg-line-strong',
      'disabled:cursor-not-allowed disabled:opacity-45',
      size === 'lg' ? 'h-6 w-10.5' : 'h-5 w-8.5',
      props.class,
    )"
  >
    <SwitchThumb
      data-slot="switch-thumb"
      :class="cn(
        'pointer-events-none block rounded-pill transition-transform duration-150',
        'data-[state=checked]:bg-on-accent data-[state=unchecked]:translate-x-0 data-[state=unchecked]:bg-fg',
        size === 'lg' ? 'size-4.5 data-[state=checked]:translate-x-4.5' : 'size-3.5 data-[state=checked]:translate-x-3.5',
      )"
    >
      <slot
        name="thumb"
        v-bind="slotProps"
      />
    </SwitchThumb>
  </SwitchRoot>
</template>
