<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { InputVariants } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { Input } from '@/components/ui/input'

const props = defineProps<{
  font?: InputVariants['font']
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <Input
    data-slot="input-group-control"
    :font="font"
    :class="cn('h-full flex-1 bg-transparent ring-0 focus-visible:ring-0 disabled:opacity-100', props.class)"
  />
</template>
