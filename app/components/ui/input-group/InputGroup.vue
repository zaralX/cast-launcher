<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { InputGroupVariants } from '.'
import { cn } from '@/lib/utils'
import { INPUT_GROUP_SIZE, inputGroupVariants } from '.'

const props = defineProps<{
  size?: InputGroupVariants['size']
  class?: HTMLAttributes['class']
}>()

provide(INPUT_GROUP_SIZE, toRef(props, 'size'))
</script>

<template>
  <div
    data-slot="input-group"
    role="group"
    :class="cn(inputGroupVariants({ size }), props.class)"
  >
    <slot />
  </div>
</template>
