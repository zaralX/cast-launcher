<script setup lang="ts">
defineProps<{
  label?: string
  description?: string
}>()
</script>

<template>
  <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
    <div class="min-w-0">
      <p
        v-if="label"
        class="mb-2 font-mono text-label uppercase tracking-caps text-fg-faint"
      >
        {{ label }}
      </p>
      <div class="text-body leading-relaxed text-fg-muted">
        <slot name="description">
          {{ description }}
        </slot>
      </div>
    </div>

    <div class="flex shrink-0 items-center gap-2">
      <slot />
    </div>
  </div>
</template>
