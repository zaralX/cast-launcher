<script setup lang="ts">
const model = defineModel<number>()
const props = defineProps<{
  steps: string[]
}>()

const current = computed(() => Math.min(Math.max(model.value ?? 0, 0), props.steps.length))
const percent = computed(() => Math.round((current.value / props.steps.length) * 100))
</script>

<template>
  <div data-tauri-drag-region class="flex h-screen justify-center items-center px-12 xl:px-20">
    <div class="w-full max-w-[26rem]">
      <div class="flex items-baseline gap-3">
        <p class="font-unbounded text-[20px] font-bold leading-none tracking-[-0.06em] text-fg">
          CAST<span class="text-acid">.</span>
        </p>
        <span class="font-mono text-[10px] uppercase tracking-[0.3em] text-fg-faint">Launcher</span>
      </div>

      <div class="mt-14 flex items-end justify-between">
        <p class="font-mono text-[10px] uppercase tracking-[0.28em] text-fg-faint">
          Инициализация
        </p>
        <p class="font-unbounded text-[44px] font-bold leading-[0.8] tracking-[-0.06em] text-fg">
          {{ percent }}<span class="text-[18px] text-acid">%</span>
        </p>
      </div>

      <div class="mt-5 h-px w-full bg-line">
        <div
            class="h-px bg-acid transition-[width] duration-700 ease-deck"
            :style="{ width: `${percent}%` }"
        />
      </div>

      <ol class="mt-10 space-y-2.5">
        <li
            v-for="(step, i) in steps"
            :key="step"
            class="flex items-center gap-3 font-mono text-[11px] transition-all duration-500 ease-deck"
            :class="i + 1 === current
              ? 'text-acid'
              : i + 1 < current ? 'text-fg-faint' : 'text-fg-faint/35'"
        >
          <span class="w-5 shrink-0 tabular-nums">{{ String(i + 1).padStart(2, "0") }}</span>
          <span class="truncate">{{ step }}</span>
          <span v-if="i + 1 === current" class="h-3 w-1.5 bg-acid animate-blink"/>
          <UIcon v-else-if="i + 1 < current" name="i-lucide-check" class="size-3 shrink-0"/>
        </li>
      </ol>
    </div>
  </div>
</template>
