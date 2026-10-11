<script setup lang="ts">
defineProps<{
  title: string
  description?: string
}>()
</script>

<template>
  <div class="flex items-end gap-4">
    <div class="min-w-0 flex-1">
      <KitTitle>{{ title }}</KitTitle>
      <p
        v-if="description"
        class="mt-1.5 text-body text-fg-muted"
      >
        {{ description }}
      </p>
    </div>
    <div
      v-if="$slots.default"
      class="flex shrink-0 items-center gap-2"
    >
      <slot />
    </div>
  </div>
</template>
