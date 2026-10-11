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
    <slot />
  </RadioGroupItem>
</template>
