<script setup lang="ts">
defineProps<{
  to: string
  icon: string
  label: string
}>()

const route = useRoute()
</script>

<template>
  <Tooltip>
    <TooltipTrigger as-child>
      <NuxtLink
        :to="to"
        :aria-label="label"
        class="group relative grid size-14 place-items-center outline-none"
      >
        <span
          class="absolute top-1/2 left-0 w-px -translate-y-1/2 bg-acid transition-all duration-500 ease-deck"
          :class="route.path === to ? 'h-8 opacity-100' : 'h-0 opacity-0 group-hover:h-4 group-hover:opacity-60 group-focus-visible:h-4 group-focus-visible:opacity-60'"
        />

        <Icon
          :name="icon"
          class="size-[18px] transition-all duration-300 ease-deck"
          :class="route.path === to
            ? 'text-acid'
            : 'text-fg-faint group-hover:-translate-y-0.5 group-hover:text-fg'"
        />
      </NuxtLink>
    </TooltipTrigger>
    <TooltipContent side="right">
      {{ label }}
    </TooltipContent>
  </Tooltip>
</template>
