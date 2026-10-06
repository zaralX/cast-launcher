<script setup lang="ts">
import type { RadioGroupItemProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { RadioGroupItem, useForwardProps } from 'reka-ui'
import { cn } from '@/lib/utils'
import { RADIO_GROUP_VARIANT, radioGroupItemVariants } from '.'

const props = defineProps<RadioGroupItemProps & { class?: HTMLAttributes['class'] }>()

const variant = inject(RADIO_GROUP_VARIANT, ref('segmented'))

const delegatedProps = reactiveOmit(props, 'class')

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <RadioGroupItem
    data-slot="radio-group-item"
    v-bind="forwardedProps"
    :class="cn(radioGroupItemVariants({ variant }), props.class)"
  >
    <span
      v-if="variant === 'segmented'"
      class="absolute inset-x-0 top-0 h-px origin-center scale-x-0 bg-acid transition-transform duration-500 ease-deck group-data-[state=checked]:scale-x-100"
      aria-hidden="true"
    />
    <slot />
  </RadioGroupItem>
</template>
