<script setup lang="ts">
import type { RadioGroupRootEmits, RadioGroupRootProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import type { RadioGroupVariants } from '.'
import { reactiveOmit } from '@vueuse/core'
import { RadioGroupRoot, useForwardPropsEmits } from 'reka-ui'
import { cn } from '@/lib/utils'
import { RADIO_GROUP_VARIANT, radioGroupVariants } from '.'

const props = withDefaults(defineProps<RadioGroupRootProps & {
  variant?: RadioGroupVariants['variant']
  class?: HTMLAttributes['class']
}>(), {
  variant: 'segmented',
  orientation: 'horizontal',
})
const emits = defineEmits<RadioGroupRootEmits>()

provide(RADIO_GROUP_VARIANT, toRef(props, 'variant'))

const delegatedProps = reactiveOmit(props, 'class', 'variant')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <RadioGroupRoot
    v-slot="slotProps"
    data-slot="radio-group"
    :class="cn(radioGroupVariants({ variant }), props.class)"
    v-bind="forwarded"
  >
    <slot v-bind="slotProps" />
  </RadioGroupRoot>
</template>
