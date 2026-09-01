<script setup lang="ts">
const SPARKS = [
  {top: "18%", left: "12%", delay: "0s", size: "size-1"},
  {top: "62%", left: "22%", delay: "0.7s", size: "size-1.5"},
  {top: "34%", left: "68%", delay: "1.4s", size: "size-1"},
  {top: "76%", left: "81%", delay: "2.1s", size: "size-1"},
  {top: "12%", left: "88%", delay: "2.8s", size: "size-1.5"}
]
</script>

<template>
  <div class="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
    <div
        class="absolute inset-0 opacity-[0.55]"
        style="background-image:
          linear-gradient(to right, var(--cast-line) 1px, transparent 1px),
          linear-gradient(to bottom, var(--cast-line) 1px, transparent 1px);
          background-size: 72px 72px;
          mask-image: radial-gradient(ellipse 80% 70% at 50% 40%, #000 20%, transparent 100%)"
    />

    <span class="absolute -left-40 -top-40 size-[32rem] rounded-full bg-acid/[0.10] blur-[140px] animate-drift"/>
    <span class="absolute -bottom-48 -right-32 size-[28rem] rounded-full bg-acid/[0.06] blur-[130px] animate-drift-slow"/>

    <span
        class="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-acid/35 to-transparent animate-scanline"
    />

    <span
        v-for="(spark, i) in SPARKS"
        :key="i"
        class="absolute bg-acid/60 animate-breathe"
        :class="spark.size"
        :style="{ top: spark.top, left: spark.left, animationDelay: spark.delay }"
    />
  </div>
</template>
