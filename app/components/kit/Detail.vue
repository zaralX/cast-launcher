<script setup lang="ts">
defineProps<{
  label: string
  mono?: boolean
  title?: string
}>()
</script>

<template>
  <div class="min-w-0">
    <dt class="text-caption text-fg-faint">
      {{ label }}
    </dt>
    <dd
      class="mt-0.5 truncate text-fg"
      :class="mono ? 'font-mono text-caption' : 'text-body'"
      :title="title"
    >
      <slot />
    </dd>
  </div>
</template>
