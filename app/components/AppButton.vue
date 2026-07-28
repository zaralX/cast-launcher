<script setup lang="ts">
withDefaults(defineProps<{
  tone?: "fill" | "quiet"
}>(), {tone: "fill"})

const FILL = "relative isolate justify-center overflow-hidden border border-line text-fg " +
    "before:absolute before:inset-0 before:-z-10 before:origin-left before:scale-x-0 before:bg-acid " +
    "before:transition-transform before:duration-500 before:ease-deck " +
    "hover:border-acid hover:text-on-acid hover:before:scale-x-100"

const QUIET = "px-0 text-fg-muted hover:bg-transparent hover:text-acid"
</script>

<template>
  <UButton
      color="neutral"
      variant="ghost"
      :class="tone === 'quiet' ? QUIET : FILL"
  >
    <template v-for="(_, name) in $slots" :key="name" #[name]="scope">
      <slot :name="name" v-bind="scope ?? {}"/>
    </template>
  </UButton>
</template>
