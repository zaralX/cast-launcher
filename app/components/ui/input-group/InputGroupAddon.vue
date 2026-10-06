<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { InputGroupVariants } from '.'
import { cn } from '@/lib/utils'
import { inputGroupAddonVariants } from '.'

const props = withDefaults(defineProps<{
  align?: InputGroupVariants['align']
  class?: HTMLAttributes['class']
}>(), {
  align: 'inline-start',
})

function focusInput(e: MouseEvent) {
  const target = e.target as HTMLElement | null
  if (target?.closest('button')) return
  const group = (e.currentTarget as HTMLElement | null)?.parentElement
  group?.querySelector('input')?.focus()
}
</script>

<template>
  <div
    data-slot="input-group-addon"
    :data-align="props.align"
    :class="cn(inputGroupAddonVariants({ align: props.align }), props.class)"
    @click="focusInput"
  >
    <slot />
  </div>
</template>
