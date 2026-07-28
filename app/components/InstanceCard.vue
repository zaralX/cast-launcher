<script setup lang="ts">
import type {Instance, InstanceType} from "~/types/instance";

const props = defineProps<{
  instance: Instance
  running?: boolean
  installing?: boolean
  progress?: number
  phase?: string
}>()

const emit = defineEmits<{
  install: [id: string]
  run: [id: string]
}>()

const TYPE_MARK: Record<InstanceType, string> = {
  vanilla: "VA",
  fabric: "FA",
  forge: "FO"
}

const mark = computed(() => TYPE_MARK[props.instance.type] ?? "??")

const state = computed<"running" | "installing" | "ready" | "absent">(() => {
  if (props.running) return "running"
  if (props.installing) return "installing"
  return props.instance.installed ? "ready" : "absent"
})
</script>

<template>
  <article
      class="cut-16 group relative flex flex-col border border-line bg-ink-800 p-5 transition-all duration-500 ease-deck hover:-translate-y-1 hover:border-acid/40 hover:bg-ink-700"
  >
    <span
        class="pointer-events-none absolute right-0 top-0 h-[23px] w-px origin-top-right rotate-45 bg-line transition-colors duration-500 group-hover:bg-acid/40"
        aria-hidden="true"
    />

    <header class="flex items-start justify-between gap-3">
      <span
          class="grid size-8 shrink-0 place-items-center border border-line font-mono text-[10px] tracking-[0.08em] text-fg-faint transition-colors duration-500 group-hover:border-acid/40 group-hover:text-acid"
      >
        {{ mark }}
      </span>

      <div class="flex flex-col items-end gap-1 pr-3.5">
        <span class="font-mono text-[10px] leading-none text-fg-muted">{{ instance.minecraftVersion }}</span>
        <span
            v-if="instance.loaderVersion"
            class="max-w-[9rem] truncate font-mono text-[9px] leading-none text-fg-faint"
        >
          {{ instance.loaderVersion }}
        </span>
      </div>
    </header>

    <div class="mt-6 min-w-0">
      <h3
          class="truncate font-unbounded text-[15px] font-semibold leading-tight tracking-[-0.035em] text-fg"
          :title="instance.name"
      >
        {{ instance.name }}
      </h3>
      <p class="mt-2 line-clamp-2 min-h-[2.25rem] text-[12px] leading-relaxed text-fg-muted">
        {{ instance.description || "Без описания" }}
      </p>
    </div>

    <div class="mt-5 h-px w-full bg-line transition-colors duration-500 group-hover:bg-line-strong"/>

    <footer class="mt-4">
      <div v-if="state === 'running'" class="flex h-9 items-center gap-2.5 px-1">
        <span class="relative grid size-2 place-items-center">
          <span class="absolute size-2 bg-acid animate-breathe"/>
          <span class="size-2 bg-acid"/>
        </span>
        <span class="font-mono text-[11px] uppercase tracking-[0.18em] text-acid">Запущено</span>
      </div>

      <div v-else-if="state === 'installing'" class="flex h-9 flex-col justify-center gap-2 px-1">
        <span class="flex items-baseline justify-between gap-3">
          <span class="min-w-0 truncate font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted">
            {{ phase || 'Установка' }}
          </span>
          <span v-if="progress != null" class="shrink-0 font-mono text-[10px] tabular-nums text-acid">
            {{ Math.round(progress * 100) }}%
          </span>
        </span>

        <span class="relative block h-px w-full overflow-hidden bg-line">
          <span
              v-if="progress == null"
              class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"
          />
          <span
              v-else
              class="absolute inset-y-0 left-0 bg-acid transition-[width] duration-500 ease-deck"
              :style="{ width: `${Math.round(progress * 100)}%` }"
          />
        </span>
      </div>

      <AppButton
          v-else
          block
          class="group/act h-9 tracking-[0.18em]"
          @click="state === 'ready' ? emit('run', instance.id) : emit('install', instance.id)"
      >
        <template #leading>
          <UIcon
              :name="state === 'ready' ? 'i-lucide-play' : 'i-lucide-arrow-down-to-line'"
              class="size-3.5 transition-transform duration-500 ease-deck group-hover/act:translate-x-0.5"
          />
        </template>
        {{ state === 'ready' ? 'Играть' : 'Загрузить' }}
      </AppButton>
    </footer>
  </article>
</template>
