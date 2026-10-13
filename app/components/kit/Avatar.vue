<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  name?: string | null
  muted?: boolean
  class?: HTMLAttributes['class']
}>(), {
  name: null,
  muted: false,
  class: undefined,
})
</script>

<template>
  <span :class="cn('grid size-8 shrink-0 place-items-center overflow-hidden rounded-sm bg-control text-fg-faint', props.class)">
    <img
      v-if="name"
      :key="name"
      :src="faceUrl(name)"
      :alt="name"
      class="size-full [image-rendering:pixelated]"
      :class="muted && 'opacity-70 grayscale'"
      @error="fallbackFace"
    >
    <Icon
      v-else
      name="i-lucide-user-round"
      class="size-1/2"
    />
  </span>
</template>
