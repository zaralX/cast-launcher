<script setup lang="ts">
import type { ComboboxInputEmits, ComboboxInputProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ComboboxInput, useForwardPropsEmits } from 'reka-ui'
import { cn } from '@/lib/utils'

defineOptions({
  inheritAttrs: false,
})

const props = defineProps<ComboboxInputProps & {
  class?: HTMLAttributes['class']
}>()

const emits = defineEmits<ComboboxInputEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <div
    data-slot="combobox-input-wrapper"
    class="flex h-8 items-center gap-2 border-b border-line px-2.5"
  >
    <Icon
      name="i-lucide-search"
      class="size-3.5 shrink-0 text-fg-faint"
    />
    <ComboboxInput
      data-slot="combobox-input"
      :class="cn('h-full w-full bg-transparent text-lead text-fg outline-none placeholder:text-fg-faint disabled:cursor-not-allowed', props.class)"
      v-bind="{ ...$attrs, ...forwarded }"
    >
      <slot />
    </ComboboxInput>
  </div>
</template>
