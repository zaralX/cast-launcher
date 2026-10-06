<script setup lang="ts">
import type { CheckboxRootEmits, CheckboxRootProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { CheckboxIndicator, CheckboxRoot, useForwardPropsEmits } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = defineProps<CheckboxRootProps & { class?: HTMLAttributes['class'] }>()
const emits = defineEmits<CheckboxRootEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <CheckboxRoot
    v-slot="slotProps"
    data-slot="checkbox"
    v-bind="forwarded"
    :class="cn(
      'peer size-4 shrink-0 cursor-pointer overflow-hidden rounded-sm ring ring-inset ring-line-strong outline-none',
      'data-[state=checked]:bg-acid data-[state=checked]:text-on-acid data-[state=checked]:ring-acid',
      'data-[state=indeterminate]:bg-acid data-[state=indeterminate]:text-on-acid data-[state=indeterminate]:ring-acid',
      'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acid disabled:cursor-not-allowed disabled:opacity-75',
      props.class,
    )"
  >
    <CheckboxIndicator
      data-slot="checkbox-indicator"
      class="grid size-full place-content-center"
    >
      <slot v-bind="slotProps">
        <Icon
          :name="slotProps.state === 'indeterminate' ? 'i-lucide-minus' : 'i-lucide-check'"
          class="size-3.5"
        />
      </slot>
    </CheckboxIndicator>
  </CheckboxRoot>
</template>
