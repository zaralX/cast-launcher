<script setup lang="ts">
</script>

<template>
  <div class="relative h-screen w-full overflow-hidden bg-ink-900 text-fg">
    <div
        class="pointer-events-none absolute -left-40 -top-40 h-[26rem] w-[26rem] rounded-full bg-acid/[0.07] blur-[120px]"
        aria-hidden="true"
    />
    <slot/>
  </div>
</template>
