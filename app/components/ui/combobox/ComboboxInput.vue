<script setup lang="ts">
import type { ComboboxInputEmits, ComboboxInputProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ComboboxInput, useForwardPropsEmits } from 'reka-ui'
import { cn } from '@/lib/utils'
import { InputGroup, InputGroupAddon } from '@/components/ui/input-group'

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
    class="p-1 pb-0"
  >
    <InputGroup>
      <InputGroupAddon>
        <Icon
          name="i-lucide-search"
          class="size-3.5"
        />
      </InputGroupAddon>
      <ComboboxInput
        data-slot="input-group-control"
        :class="cn('h-full min-w-0 flex-1 bg-transparent pr-2.5 pl-2 text-fg outline-none placeholder:text-fg-faint', props.class)"
        v-bind="{ ...$attrs, ...forwarded }"
      >
        <slot />
      </ComboboxInput>
    </InputGroup>
  </div>
</template>
