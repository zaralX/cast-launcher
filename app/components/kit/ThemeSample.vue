<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { ThemePreview } from '~/types/app'
import { cn } from '@/lib/utils'

const props = defineProps<{
  preview: ThemePreview
  class?: HTMLAttributes['class']
}>()

const square = computed(() => props.preview.square && 'rounded-none')
</script>

<template>
  <span
    :class="props.class"
    :style="{ background: preview.frame }"
  >
    <span
      :class="cn('absolute top-2 right-0 bottom-0 left-3 grid grid-cols-2 content-start gap-1 rounded-tl-sm p-1.5', square)"
      :style="{
        background: preview.sheet,
        boxShadow: preview.sheetEdge ? `inset 1px 1px 0 ${preview.sheetEdge}` : undefined,
      }"
    >
      <span
        v-for="tile in 2"
        :key="tile"
        :class="cn('block h-4 rounded-xs', square)"
        :style="{
          background: preview.panel,
          boxShadow: preview.cardLine ? `inset 0 0 0 1px ${preview.cardLine}` : undefined,
        }"
      />
      <span
        :class="cn('block h-1.5 w-3/5 rounded-xs', square)"
        :style="{ background: preview.accent }"
      />
    </span>
  </span>
</template>
