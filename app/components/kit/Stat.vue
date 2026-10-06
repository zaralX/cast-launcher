<script setup lang="ts">
defineProps<{
  label: string
  value?: string
}>()
</script>

<template>
  <div class="min-w-0 border-line px-4 py-3">
    <dt class="font-mono text-micro uppercase tracking-caps text-fg-faint">
      {{ label }}
    </dt>
    <dd
      class="mt-1.5 truncate font-unbounded text-title tracking-heading text-fg"
      :title="value"
    >
      <slot>{{ value }}</slot>
    </dd>
  </div>
</template>
