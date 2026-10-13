<script setup lang="ts">
import type { SelectLabelProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { SelectLabel } from 'reka-ui'
import { MENU_LABEL } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<SelectLabelProps & { class?: HTMLAttributes['class'] }>()
</script>

<template>
  <SelectLabel
    data-slot="select-label"
    :class="cn(MENU_LABEL, props.class)"
  >
    <slot />
  </SelectLabel>
</template>
