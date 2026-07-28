<script setup lang="ts">
defineProps<{
  index: string
  title: string
  meta?: string
}>()
</script>

<template>
  <div class="flex items-center gap-4">
    <span class="font-mono text-[10px] leading-none tracking-[0.3em] text-acid">{{ index }}</span>
    <h2 class="font-unbounded text-[12px] font-semibold uppercase leading-none tracking-[0.2em] text-fg">
      {{ title }}
    </h2>
    <span class="h-px flex-1 bg-line"/>
    <span v-if="meta" class="font-mono text-[10px] leading-none tracking-[0.12em] text-fg-faint">{{ meta }}</span>
    <slot name="action"/>
  </div>
</template>
