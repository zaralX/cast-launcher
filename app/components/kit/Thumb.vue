<script setup lang="ts">
withDefaults(defineProps<{
  src?: string | null
  alt?: string
  size?: 'sm' | 'md' | 'lg'
  fallbackIcon?: string
}>(), {
  src: undefined,
  alt: '',
  size: 'md',
  fallbackIcon: 'i-lucide-package',
})

const SIZE = {
  sm: 'size-10',
  md: 'size-12',
  lg: 'size-14',
}
</script>

<template>
  <span
    class="grid shrink-0 place-items-center overflow-hidden border border-line bg-ink-900"
    :class="SIZE[size]"
  >
    <img
      v-if="src"
      :src="src"
      :alt="alt"
      loading="lazy"
      class="size-full object-cover"
    >
    <slot v-else>
      <Icon
        :name="fallbackIcon"
        class="size-5 text-fg-faint"
      />
    </slot>
  </span>
</template>
