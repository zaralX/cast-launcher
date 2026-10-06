<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { AlertVariants } from '.'
import { cn } from '@/lib/utils'
import { alertIconVariants, alertVariants } from '.'

const props = defineProps<{
  variant?: AlertVariants['variant']
  icon?: string
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div
    data-slot="alert"
    role="alert"
    :class="cn(alertVariants({ variant }), props.class)"
  >
    <Icon
      v-if="icon"
      :name="icon"
      :class="alertIconVariants({ variant })"
    />
    <div class="min-w-0 flex-1">
      <slot />
    </div>
    <div
      v-if="$slots.action"
      class="shrink-0 self-center"
    >
      <slot name="action" />
    </div>
  </div>
</template>
