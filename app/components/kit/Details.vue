<script setup lang="ts">
withDefaults(defineProps<{
  columns?: 1 | 2
}>(), {
  columns: 2,
})
</script>

<template>
  <dl
    class="grid gap-x-8 gap-y-2"
    :class="columns === 2 && 'grid-cols-2'"
  >
    <slot />
  </dl>
</template>
