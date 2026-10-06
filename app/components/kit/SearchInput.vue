<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { InputVariants } from '@/components/ui/input'

defineOptions({
  inheritAttrs: false,
})

const props = defineProps<{
  loading?: boolean
  icon?: string
  size?: 'md' | 'lg'
  font?: InputVariants['font']
  class?: HTMLAttributes['class']
}>()

const model = defineModel<string>()
</script>

<template>
  <InputGroup
    :size="size"
    :class="props.class"
  >
    <InputGroupAddon>
      <Spinner
        v-if="loading"
        class="size-3.5"
      />
      <Icon
        v-else
        :name="icon ?? 'i-lucide-search'"
        class="size-3.5"
      />
    </InputGroupAddon>
    <InputGroupInput
      v-model="model"
      :font="font"
      v-bind="$attrs"
    />
  </InputGroup>
</template>
