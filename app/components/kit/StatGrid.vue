<script setup lang="ts">
withDefaults(defineProps<{
  columns?: 2 | 3
}>(), {
  columns: 2,
})
</script>

<template>
  <dl
    class="grid border border-line"
    :class="columns === 3
      ? 'grid-cols-3 [&>*:not(:nth-child(3n+1))]:border-l'
      : 'grid-cols-2 [&>*:nth-child(2n)]:border-l'"
  >
    <slot />
  </dl>
</template>
