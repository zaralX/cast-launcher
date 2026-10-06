<script setup lang="ts">
import type { HTMLAttributes } from 'vue'

defineOptions({
  inheritAttrs: false,
})

const props = defineProps<{
  unit?: string
  class?: HTMLAttributes['class']
}>()

const model = defineModel<number | string | null>()
</script>

<template>
  <InputGroup :class="props.class">
    <InputGroupInput
      v-model="model"
      type="number"
      font="mono"
      v-bind="$attrs"
    />
    <InputGroupAddon
      v-if="unit"
      align="inline-end"
    >
      <InputGroupText>{{ unit }}</InputGroupText>
    </InputGroupAddon>
  </InputGroup>
</template>
