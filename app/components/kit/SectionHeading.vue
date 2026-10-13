<script setup lang="ts">
defineProps<{
  title: string
  meta?: string
}>()
</script>

<template>
  <div class="flex min-h-7 items-center gap-2.5">
    <h2 class="text-title font-semibold text-fg classic:font-mono classic:text-caption classic:uppercase classic:tracking-caps">
      {{ title }}
    </h2>
    <span
      v-if="meta"
      class="text-caption text-fg-faint"
    >{{ meta }}</span>
    <div
      v-if="$slots.action"
      class="ml-auto flex items-center gap-2"
    >
      <slot name="action" />
    </div>
  </div>
</template>
