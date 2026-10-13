<script setup lang="ts">
defineProps<{
  label: string
  hint?: string
  error?: string
  for?: string
}>()
</script>

<template>
  <div>
    <Label
      :for="$props.for"
      class="mb-1.5"
    >
      {{ label }}
    </Label>
    <slot />
    <p
      v-if="error"
      class="mt-1.5 text-caption text-danger"
    >
      {{ error }}
    </p>
    <p
      v-else-if="hint"
      class="mt-1.5 text-caption text-fg-faint"
    >
      {{ hint }}
    </p>
  </div>
</template>
