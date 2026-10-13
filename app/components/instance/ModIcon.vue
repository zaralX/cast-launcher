<script setup lang="ts">
const props = withDefaults(defineProps<{
  iconKey?: string | null
  fallbackUrl?: string | null
  name: string
  size?: 'sm' | 'md'
}>(), { size: 'sm' })

const modsStore = useModsStore()

const SIZES = {
  sm: 'size-9',
  md: 'size-12',
}

const url = computed(() => modsStore.iconOf(props.iconKey) || props.fallbackUrl || null)

const mark = computed(() => props.name.trim().slice(0, 2).toUpperCase() || '??')
</script>

<template>
  <span
    class="grid shrink-0 place-items-center overflow-hidden rounded-sm bg-control"
    :class="SIZES[size]"
  >
    <img
      v-if="url"
      :src="url"
      alt=""
      class="size-full object-cover"
    >
    <span
      v-else
      class="text-micro font-semibold text-fg-muted"
    >{{ mark }}</span>
  </span>
</template>
