<script setup lang="ts">
import type { TabsTriggerProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { TabsTrigger, useForwardProps } from 'reka-ui'
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<TabsTriggerProps & {
  icon?: string
  meta?: string | number
  class?: HTMLAttributes['class']
}>()

const delegatedProps = reactiveOmit(props, 'class', 'icon', 'meta')

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <TabsTrigger
    data-slot="tabs-trigger"
    :class="cn(
      FOCUS_RING,
      'group relative flex cursor-pointer items-center gap-2.5 font-medium whitespace-nowrap text-fg-muted transition-colors duration-150',
      'hover:text-fg data-[state=active]:text-fg disabled:pointer-events-none disabled:opacity-45',
      'data-[orientation=horizontal]:self-stretch',
      'data-[orientation=vertical]:h-7.5 data-[orientation=vertical]:w-full data-[orientation=vertical]:rounded-sm data-[orientation=vertical]:px-2.5',
      'data-[orientation=vertical]:hover:bg-control data-[orientation=vertical]:data-[state=active]:bg-control',
      props.class,
    )"
    v-bind="forwardedProps"
  >
    <Icon
      v-if="icon"
      :name="icon"
      class="size-4 shrink-0 text-fg-faint group-data-[state=active]:text-accent"
    />
    <span class="min-w-0 flex-1 truncate text-left">
      <slot />
    </span>
    <span
      v-if="meta != null"
      class="shrink-0 text-caption font-normal text-fg-faint tabular-nums"
    >{{ meta }}</span>
    <span
      class="absolute inset-x-0 -bottom-px h-0.5 scale-x-0 rounded-pill bg-accent transition-transform duration-200 group-data-[orientation=vertical]:hidden group-data-[state=active]:scale-x-100"
      aria-hidden="true"
    />
  </TabsTrigger>
</template>
