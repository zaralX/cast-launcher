<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <Icon
    name="i-lucide-loader-circle"
    role="status"
    :aria-label="$t('common.loading')"
    :class="cn('size-4 shrink-0 animate-spin', props.class)"
  />
</template>
