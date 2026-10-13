<script setup lang="ts">
import type { TabsListProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { TabsList } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = defineProps<TabsListProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <TabsList
    data-slot="tabs-list"
    v-bind="delegatedProps"
    :class="cn(
      'flex',
      'data-[orientation=horizontal]:min-h-9 data-[orientation=horizontal]:gap-5.5',
      'data-[orientation=vertical]:flex-col data-[orientation=vertical]:gap-0.5',
      props.class,
    )"
  >
    <slot />
  </TabsList>
</template>
