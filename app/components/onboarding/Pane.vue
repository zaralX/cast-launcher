<script setup lang="ts">
defineProps<{
  index: string
  title: string
  subtitle?: string
}>()
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <header class="shrink-0 px-10 pt-8 xl:px-14">
      <p class="animate-rise font-mono text-[10px] uppercase tracking-[0.4em] text-acid">{{ index }}</p>

      <h2
          class="animate-rise [animation-delay:60ms] mt-3 font-unbounded text-[clamp(21px,2.4vw,28px)] font-bold leading-[0.95] tracking-[-0.055em] text-fg"
      >
        {{ title }}<span class="text-acid">.</span>
      </h2>

      <p
          v-if="subtitle"
          class="animate-rise [animation-delay:120ms] mt-3 max-w-[44rem] text-[12px] leading-relaxed text-fg-muted"
      >
        {{ subtitle }}
      </p>
    </header>

    <div class="animate-rise [animation-delay:180ms] min-h-0 flex-1 overflow-y-auto px-10 py-7 xl:px-14">
      <slot/>
    </div>
  </div>
</template>
