<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { ButtonVariants } from '@/components/ui/button'
import { Button } from '@/components/ui/button'

const props = withDefaults(defineProps<{
  variant?: ButtonVariants['variant']
  size?: ButtonVariants['size']
  icon?: string
  class?: HTMLAttributes['class']
}>(), {
  variant: 'ghost',
  size: 'icon-sm',
  icon: undefined,
  class: undefined,
})
</script>

<template>
  <Button
    data-slot="input-group-button"
    :variant="variant"
    :size="size"
    :icon="icon"
    :class="props.class"
  >
    <slot />
  </Button>
</template>
