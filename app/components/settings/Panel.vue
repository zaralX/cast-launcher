<script setup lang="ts">
defineProps<{
  index: string
  title: string
  description?: string
  icon?: string
}>()
</script>

<template>
  <section class="group relative border border-line bg-ink-800 transition-colors duration-500 hover:border-line-strong">
    <header class="flex items-start gap-5 border-b border-line px-6 py-5">
      <span class="mt-0.5 font-mono text-[10px] leading-none tracking-[0.24em] text-acid">{{ index }}</span>

      <div class="min-w-0 flex-1">
        <h2 class="font-unbounded text-[12px] font-semibold uppercase leading-none tracking-[0.2em] text-fg">
          {{ title }}
        </h2>
        <p v-if="description" class="mt-2.5 text-[12px] leading-relaxed text-fg-muted">{{ description }}</p>
      </div>

      <UIcon
          v-if="icon"
          :name="icon"
          class="size-4 shrink-0 text-fg-faint transition-colors duration-500 group-hover:text-fg-muted"
      />
    </header>

    <div class="px-6 py-7">
      <slot/>
    </div>
  </section>
</template>
