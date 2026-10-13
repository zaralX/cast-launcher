<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  open: boolean
  label: string
  position?: 'center' | 'end'
  class?: HTMLAttributes['class']
}>(), {
  position: 'end',
  class: undefined,
})
</script>

<template>
  <Transition
    enter-active-class="transition duration-200 ease-deck"
    leave-active-class="transition duration-150"
    enter-from-class="translate-y-3 opacity-0"
    leave-to-class="translate-y-3 opacity-0"
  >
    <div
      v-if="open"
      role="status"
      :class="cn(
        'fixed bottom-5 z-40 flex items-center gap-1.5 rounded-md bg-raise py-1.5 pr-1.5 pl-4 shadow-float ring-1 ring-line-strong',
        position === 'center' ? 'left-[calc(50%+1.75rem)] -translate-x-1/2' : 'right-7',
        props.class,
      )"
    >
      <span class="mr-2 text-body font-medium text-fg">{{ label }}</span>
      <slot />
    </div>
  </Transition>
</template>
