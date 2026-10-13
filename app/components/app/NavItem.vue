<script setup lang="ts">
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<{
  to: string
  icon: string
  label: string
  match?: string[]
}>()

const route = useRoute()

const active = computed(() => [props.to, ...(props.match ?? [])]
  .some(path => route.path === path || route.path.startsWith(`${path}/`)))
</script>

<template>
  <Tooltip>
    <TooltipTrigger as-child>
      <NuxtLink
        :to="to"
        :aria-label="label"
        :aria-current="active ? 'page' : undefined"
        :class="cn(
          FOCUS_RING,
          'grid size-10 place-items-center rounded-md transition-colors duration-150',
          active ? 'bg-accent/18 text-accent' : 'text-fg-faint hover:bg-control hover:text-fg',
        )"
      >
        <Icon
          :name="icon"
          class="size-5"
        />
      </NuxtLink>
    </TooltipTrigger>
    <TooltipContent side="right">
      {{ label }}
    </TooltipContent>
  </Tooltip>
</template>
