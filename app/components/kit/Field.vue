<script setup lang="ts">
defineProps<{
  label: string
  hint?: string
  for?: string
}>()
</script>

<template>
  <div>
    <Label
      :for="$props.for"
      class="mb-2"
    >
      {{ label }}
    </Label>
    <slot />
    <p
      v-if="hint"
      class="mt-2 font-mono text-label leading-relaxed text-fg-faint/70"
    >
      {{ hint }}
    </p>
  </div>
</template>
