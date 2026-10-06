<script lang="ts" setup>
import type { ToasterProps } from 'vue-sonner'
import { Toaster as Sonner } from 'vue-sonner'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<ToasterProps>(), {
  position: 'bottom-right',
  gap: 12,
  visibleToasts: 5,
  offset: 16,
})
</script>

<template>
  <Sonner
    v-bind="props"
    :class="cn('group/toaster', props.class)"
    :style="{ '--width': '24rem' }"
  />
</template>
