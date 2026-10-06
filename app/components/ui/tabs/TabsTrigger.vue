<script setup lang="ts">
import type { TabsTriggerProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { TabsTrigger, useForwardProps } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = defineProps<TabsTriggerProps & {
  icon?: string
  meta?: string
  class?: HTMLAttributes['class']
}>()

const delegatedProps = reactiveOmit(props, 'class', 'icon', 'meta')

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <TabsTrigger
    data-slot="tabs-trigger"
    :class="cn(
      'group relative flex w-full cursor-pointer items-center gap-3 border-b border-line py-3 pr-2 pl-4 text-left text-fg-faint outline-none transition-colors duration-300',
      'hover:bg-ink-700 focus-visible:bg-ink-700 data-[state=active]:text-fg disabled:pointer-events-none disabled:opacity-75',
      props.class,
    )"
    v-bind="forwardedProps"
  >
    <span
      class="absolute inset-y-0 left-0 w-[2px] scale-y-0 bg-acid transition-transform duration-500 ease-deck group-data-[state=active]:scale-y-100 group-data-[state=inactive]:group-hover:scale-y-50 group-data-[state=inactive]:group-hover:bg-line-strong"
      aria-hidden="true"
    />
    <Icon
      v-if="icon"
      :name="icon"
      class="size-4 shrink-0 group-data-[state=active]:text-acid"
    />
    <span class="flex-1 font-mono text-caption uppercase tracking-caps">
      <slot />
    </span>
    <span
      v-if="meta"
      class="font-mono text-micro tracking-caps text-fg-faint/60"
    >{{ meta }}</span>
  </TabsTrigger>
</template>
