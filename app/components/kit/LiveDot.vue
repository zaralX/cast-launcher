<script setup lang="ts">
withDefaults(defineProps<{
  size?: 'sm' | 'md'
}>(), {
  size: 'md',
})
</script>

<template>
  <span
    class="relative grid shrink-0 place-items-center"
    :class="size === 'sm' ? 'size-1.5' : 'size-2'"
  >
    <span class="absolute size-full bg-current animate-breathe" />
    <span class="size-full bg-current" />
  </span>
</template>
