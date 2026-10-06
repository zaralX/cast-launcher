<script setup lang="ts">
withDefaults(defineProps<{
  eyebrow?: string
  title: string
  description?: string
  size?: 'xs' | 'lg'
}>(), {
  eyebrow: undefined,
  description: undefined,
  size: 'lg',
})
</script>

<template>
  <div>
    <p
      v-if="eyebrow"
      class="mb-3 font-mono text-label uppercase tracking-caps-wide text-fg-faint"
    >
      {{ eyebrow }}
    </p>
    <KitTitle :size="size">
      {{ title }}
    </KitTitle>
    <p
      v-if="description"
      class="mt-5 text-body leading-relaxed text-fg-muted"
    >
      {{ description }}
    </p>
  </div>
</template>
