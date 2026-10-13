<script setup lang="ts">
withDefaults(defineProps<{
  src?: string | null
  alt?: string
  size?: 'sm' | 'md' | 'lg' | 'xl'
  fallbackIcon?: string
}>(), {
  src: undefined,
  alt: '',
  size: 'md',
  fallbackIcon: 'i-lucide-package',
})

const SIZE = {
  sm: 'size-10 rounded-sm',
  md: 'size-12 rounded-sm',
  lg: 'size-14 rounded-sm',
  xl: 'size-16 rounded-md',
}
</script>

<template>
  <span
    class="grid shrink-0 place-items-center overflow-hidden bg-control"
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
