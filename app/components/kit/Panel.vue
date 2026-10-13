<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  title?: string
  description?: string
  icon?: string
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <section :class="cn('rounded-md bg-panel p-4 ring-1 ring-card-line ring-inset', props.class)">
    <header
      v-if="title || $slots.action"
      class="mb-3 flex min-h-5 items-center gap-2"
    >
      <Icon
        v-if="icon"
        :name="icon"
        class="size-3.5 shrink-0 text-fg-faint"
      />
      <h2 class="min-w-0 truncate text-caption font-semibold text-fg-muted classic:font-mono classic:text-micro classic:uppercase classic:tracking-caps">
        {{ title }}
      </h2>
      <div
        v-if="$slots.action"
        class="ml-auto flex shrink-0 items-center gap-2"
      >
        <slot name="action" />
      </div>
    </header>

    <p
      v-if="description"
      class="-mt-1.5 mb-3 text-caption text-fg-faint"
    >
      {{ description }}
    </p>

    <slot />
  </section>
</template>
