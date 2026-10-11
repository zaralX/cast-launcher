<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { InputVariants } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { Input } from '@/components/ui/input'
import { INPUT_GROUP_SIZE } from '.'

const props = defineProps<{
  font?: InputVariants['font']
  class?: HTMLAttributes['class']
}>()

const size = inject(INPUT_GROUP_SIZE, ref('md'))
</script>

<template>
  <Input
    data-slot="input-group-control"
    :size="size"
    :font="font"
    :class="cn(
      'h-full flex-1 rounded-none bg-transparent px-2.5',
      'group-has-[[data-align=inline-start]]/input-group:pl-2 group-has-[[data-align=inline-end]]/input-group:pr-2',
      'focus-visible:shadow-none aria-invalid:shadow-none disabled:opacity-100',
      props.class,
    )"
  />
</template>
