<script setup lang="ts">
defineProps<{
  label: string
  value?: string
}>()
</script>

<template>
  <div class="flex items-baseline justify-between gap-3 py-1.75 text-caption">
    <dt class="shrink-0 text-fg-faint">
      {{ label }}
    </dt>
    <dd
      class="min-w-0 truncate text-right text-fg tabular-nums"
      :title="value"
    >
      <slot>{{ value }}</slot>
    </dd>
  </div>
</template>
