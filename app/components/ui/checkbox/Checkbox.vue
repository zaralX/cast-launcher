<script setup lang="ts">
import type { CheckboxRootEmits, CheckboxRootProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { CheckboxIndicator, CheckboxRoot, useForwardPropsEmits } from 'reka-ui'
import { FOCUS_RING } from '@/lib/styles'
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
      FOCUS_RING,
      'peer grid size-4 shrink-0 cursor-pointer place-items-center rounded-xs text-on-accent ring-[1.5px] ring-line-strong ring-inset transition-colors duration-150',
      'hover:ring-fg-faint data-[state=checked]:bg-accent data-[state=checked]:ring-0 data-[state=indeterminate]:bg-accent data-[state=indeterminate]:ring-0',
      'disabled:cursor-not-allowed disabled:opacity-45',
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
          class="size-3"
        />
      </slot>
    </CheckboxIndicator>
  </CheckboxRoot>
</template>
