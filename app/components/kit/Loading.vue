<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  label: string
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div :class="cn('flex flex-col items-center gap-4 py-14', props.class)">
    <Progress
      :model-value="null"
      class="w-40"
    />
    <p class="font-mono text-label uppercase tracking-caps text-fg-faint">
      {{ label }}
    </p>
  </div>
</template>
