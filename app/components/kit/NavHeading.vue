<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { MENU_LABEL } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <p :class="cn(MENU_LABEL, 'pt-3.5 pb-1.5 first:pt-1', props.class)">
    <slot />
  </p>
</template>
