<script setup lang="ts">
defineProps<{
  label: string
  hint?: string
}>()
</script>

<template>
  <div>
    <span class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ label }}</span>
    <slot/>
    <p v-if="hint" class="mt-2 font-mono text-[10px] leading-relaxed text-fg-faint/70">{{ hint }}</p>
  </div>
</template>
