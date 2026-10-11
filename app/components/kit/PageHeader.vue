<script setup lang="ts">
defineProps<{
  title: string
  meta?: string
  description?: string
}>()
</script>

<template>
  <div
    class="flex gap-4"
    :class="description ? 'items-end' : 'items-center'"
  >
    <div class="min-w-0 flex-1">
      <div class="flex min-w-0 items-baseline gap-3">
        <KitTitle class="truncate">
          {{ title }}
        </KitTitle>
        <span
          v-if="meta"
          class="shrink-0 text-body text-fg-faint"
        >{{ meta }}</span>
      </div>
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
