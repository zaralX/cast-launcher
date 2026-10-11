<script setup lang="ts">
const model = defineModel<number>()
const props = defineProps<{
  steps: string[]
}>()

const current = computed(() => Math.min(Math.max(model.value ?? 0, 0), props.steps.length))
const percent = computed(() => Math.round((current.value / props.steps.length) * 100))
</script>

<template>
  <div
    data-tauri-drag-region
    class="flex h-screen items-center justify-center bg-frame px-12"
  >
    <div class="w-full max-w-sm">
      <div class="flex items-center gap-2.5">
        <img
          src="/logo.svg"
          class="size-7"
          alt=""
        >
        <p class="font-unbounded text-heading font-semibold tracking-heading text-fg classic:uppercase">
          Cast
        </p>
      </div>

      <div class="mt-10 flex items-end justify-between gap-4">
        <p class="text-body text-fg-muted">
          {{ $t('boot.initialization') }}
        </p>
        <p class="font-unbounded text-display font-semibold tracking-display text-fg tabular-nums">
          {{ percent }}%
        </p>
      </div>

      <Progress
        :model-value="percent"
        class="mt-3"
      />

      <ol class="mt-8 space-y-2">
        <li
          v-for="(step, i) in steps"
          :key="step"
          class="flex items-center gap-2.5 text-caption transition-colors duration-300"
          :class="i + 1 === current
            ? 'text-fg'
            : i + 1 < current ? 'text-fg-faint' : 'text-fg-faint/50'"
        >
          <Spinner
            v-if="i + 1 === current"
            class="size-3.5 text-accent"
          />
          <Icon
            v-else-if="i + 1 < current"
            name="i-lucide-check"
            class="size-3.5 shrink-0 text-success"
          />
          <span
            v-else
            class="grid size-3.5 shrink-0 place-items-center"
          >
            <span class="size-1 rounded-pill bg-current" />
          </span>
          <span class="truncate">{{ step }}</span>
        </li>
      </ol>
    </div>
  </div>
</template>
