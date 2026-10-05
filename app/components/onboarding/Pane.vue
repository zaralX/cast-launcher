<script setup lang="ts">
withDefaults(defineProps<{
  title: string
  width?: 'narrow' | 'wide'
}>(), { width: 'wide' })
</script>

<template>
  <div class="h-full min-h-0 overflow-y-auto">
    <div
      class="mx-auto flex min-h-full w-full flex-col justify-center px-10 py-9 xl:px-14"
      :class="width === 'narrow' ? 'max-w-[34rem]' : 'max-w-[56rem]'"
    >
      <h2 class="font-unbounded text-[clamp(22px,2.6vw,30px)] font-bold leading-[0.95] tracking-[-0.055em] text-fg">
        {{ title }}<span class="text-acid">.</span>
      </h2>

      <div class="mt-8">
        <slot />
      </div>
    </div>
  </div>
</template>
