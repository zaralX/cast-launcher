<script setup lang="ts">
defineProps<{
  label: string
  mono?: boolean
  title?: string
}>()
</script>

<template>
  <div class="min-w-0">
    <dt class="font-mono text-micro uppercase tracking-caps text-fg-faint">
      {{ label }}
    </dt>
    <dd
      class="mt-1 truncate text-fg-muted"
      :class="mono ? 'font-mono text-caption' : 'text-body'"
      :title="title"
    >
      <slot />
    </dd>
  </div>
</template>
