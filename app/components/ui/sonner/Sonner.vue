<script lang="ts" setup>
import type { ToasterProps } from 'vue-sonner'
import { Toaster as Sonner } from 'vue-sonner'

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
    :style="{ '--width': '24rem' }"
  />
</template>
