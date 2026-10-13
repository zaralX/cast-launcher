<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<{
  active?: boolean
  activeLabel?: string
  disabled?: boolean
  class?: HTMLAttributes['class']
}>()

const emit = defineEmits<{ select: [] }>()

function select() {
  if (!props.disabled) emit('select')
}
</script>

<template>
  <li
    role="button"
    :tabindex="disabled ? -1 : 0"
    :aria-pressed="active"
    :aria-disabled="disabled || undefined"
    :class="cn(
      FOCUS_RING,
      'group relative flex items-center gap-3 rounded-md px-3 py-2.5 transition-colors duration-150',
      active ? 'bg-control' : '',
      disabled ? 'opacity-45' : 'cursor-pointer hover:bg-control',
      props.class,
    )"
    @click="select"
    @keydown.enter.self.prevent="select"
    @keydown.space.self.prevent="select"
  >
    <slot name="leading" />

    <div class="min-w-0 flex-1">
      <slot />
    </div>

    <Badge
      v-if="active && activeLabel"
      variant="accent"
    >
      {{ activeLabel }}
    </Badge>

    <slot name="trailing" />
  </li>
</template>
