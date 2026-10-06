<script setup lang="ts">
const props = withDefaults(defineProps<{
  iconKey?: string | null
  fallbackUrl?: string | null
  name: string
  size?: 'sm' | 'md'
}>(), { size: 'sm' })

const modsStore = useModsStore()

const SIZES = {
  sm: 'size-9 text-label',
  md: 'size-12 text-body',
}

const url = computed(() => modsStore.iconOf(props.iconKey) || props.fallbackUrl || null)

const mark = computed(() => props.name.trim().slice(0, 2).toUpperCase() || '??')
</script>

<template>
  <span
    class="grid shrink-0 place-items-center overflow-hidden border border-line bg-ink-900"
    :class="SIZES[size]"
  >
    <img
      v-if="url"
      :src="url"
      alt=""
      class="size-full object-contain p-[3px]"
    >
    <span
      v-else
      class="font-mono tracking-[0.08em] text-fg-faint"
    >{{ mark }}</span>
  </span>
</template>
