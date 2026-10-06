<script setup lang="ts">
withDefaults(defineProps<{
  label?: string
  description?: string
  tone?: 'default' | 'danger'
  bordered?: boolean
}>(), {
  label: undefined,
  description: undefined,
  tone: 'default',
  bordered: true,
})
</script>

<template>
  <div
    class="flex items-center justify-between gap-6"
    :class="bordered && ['border-t pt-6', tone === 'danger' ? 'border-danger/20' : 'border-line']"
  >
    <div class="min-w-0">
      <p
        v-if="label"
        class="mb-2 font-mono text-label uppercase tracking-caps"
        :class="tone === 'danger' ? 'text-danger/80' : 'text-fg-faint'"
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
