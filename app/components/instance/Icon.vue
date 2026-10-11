<script setup lang="ts">
import type { InstanceType } from '~/types/instance'

const props = withDefaults(defineProps<{
  icon?: string
  type?: InstanceType
  version?: string
  size?: 'sm' | 'md' | 'lg' | 'xl' | '2xl'
}>(), {
  icon: '',
  type: undefined,
  version: undefined,
  size: 'sm',
})

const iconStore = useIconStore()

const TYPE_MARK: Record<InstanceType, string> = {
  vanilla: 'VA',
  fabric: 'FA',
  forge: 'FO',
  neoforge: 'NF',
}

const SIZES = {
  'sm': 'size-8 rounded-xs text-micro',
  'md': 'size-12 rounded-sm text-micro',
  'lg': 'size-13 rounded-sm text-caption',
  'xl': 'size-16 rounded-md text-body',
  '2xl': 'size-22 rounded-md text-heading',
}

const mark = computed(() => (props.size !== 'sm' && props.version) || (props.type && TYPE_MARK[props.type]) || '??')
const url = computed(() => iconStore.urlOf(props.icon))

// Minecraft item icons are 16px sprites: they sit smaller in the middle and stay pixelated.
const pixelated = computed(() => props.icon.startsWith('mc-'))

watch(() => props.icon, (name) => {
  if (name) iconStore.ensureUrl(name)
}, { immediate: true })
</script>

<template>
  <span
    class="grid shrink-0 place-items-center overflow-hidden bg-control text-fg-muted"
    :class="SIZES[size]"
  >
    <img
      v-if="url"
      :src="url"
      alt=""
      class="size-full"
      :class="pixelated ? 'object-contain p-[23%] [image-rendering:pixelated]' : 'object-cover'"
    >
    <span
      v-else
      class="truncate px-0.5 font-unbounded font-semibold tracking-heading"
    >{{ mark }}</span>
  </span>
</template>
