<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div
    data-slot="skeleton"
    :class="cn('animate-pulse rounded-sm bg-control', props.class)"
  />
</template>
