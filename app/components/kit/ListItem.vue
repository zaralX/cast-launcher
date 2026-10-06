<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
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
      'group relative flex items-center gap-4 border-b border-line py-3.5 pr-1 pl-4 outline-none transition-colors duration-300',
      disabled ? 'opacity-45' : 'cursor-pointer hover:bg-ink-700 focus-visible:bg-ink-700',
      props.class,
    )"
    @click="select"
    @keydown.enter.self.prevent="select"
    @keydown.space.self.prevent="select"
  >
    <span
      class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
      :class="active ? 'scale-y-100' : disabled ? 'scale-y-0' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
    />

    <slot name="leading" />

    <div class="min-w-0 flex-1">
      <slot />
    </div>

    <span
      v-if="active && activeLabel"
      class="shrink-0 font-mono text-micro uppercase tracking-caps text-acid"
    >
      {{ activeLabel }}
    </span>

    <slot name="trailing" />
  </li>
</template>
