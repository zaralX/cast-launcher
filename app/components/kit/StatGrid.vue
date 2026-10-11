<script setup lang="ts">
withDefaults(defineProps<{
  columns?: 2 | 3
}>(), {
  columns: 2,
})
</script>

<template>
  <dl
    class="grid divide-x divide-line rounded-md bg-panel ring-1 ring-card-line ring-inset"
    :class="columns === 3 ? 'grid-cols-3' : 'grid-cols-2'"
  >
    <slot />
  </dl>
</template>
