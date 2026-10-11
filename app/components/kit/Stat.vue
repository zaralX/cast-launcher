<script setup lang="ts">
defineProps<{
  label: string
  value?: string
}>()
</script>

<template>
  <div class="min-w-0 px-4 py-3">
    <dt class="text-caption text-fg-faint">
      {{ label }}
    </dt>
    <dd
      class="mt-0.5 truncate text-title font-semibold text-fg tabular-nums"
      :title="value"
    >
      <slot>{{ value }}</slot>
    </dd>
  </div>
</template>
