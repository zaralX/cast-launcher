<script setup lang="ts">
defineProps<{
  label: string
  value: string
  icon: string
  copy?: boolean
}>()
</script>

<template>
  <button
    type="button"
    class="group relative flex cursor-pointer items-center gap-4 border border-line bg-ink-800 px-5 py-4 text-left outline-none transition-colors duration-500 hover:border-acid/50 focus-visible:border-acid/50"
  >
    <Icon
      :name="icon"
      class="size-[18px] shrink-0 text-fg-faint transition-colors duration-300 group-hover:text-acid"
    />

    <span class="min-w-0 flex-1">
      <Label
        as="span"
        class="cursor-pointer"
      >
        {{ label }}
      </Label>
      <span class="mt-1.5 block truncate text-title leading-none text-fg">{{ value }}</span>
    </span>

    <Icon
      :name="copy ? 'i-lucide-copy' : 'i-lucide-arrow-up-right'"
      class="size-3.5 shrink-0 text-fg-faint opacity-0 transition-all duration-300 ease-deck group-hover:opacity-100"
      :class="copy ? 'group-hover:text-fg-muted' : '-translate-x-1 group-hover:translate-x-0'"
    />
  </button>
</template>
