<script setup lang="ts">
import type {MyPackObject} from "~/types/pack";

const props = defineProps<{
  pack: MyPackObject
  packId: string
}>()

const emit = defineEmits<{ install: [packId: string] }>()
</script>

<template>
  <article
      class="group relative flex flex-col justify-between overflow-hidden border border-line bg-ink-800 transition-all duration-500 ease-deck hover:border-acid/40 hover:bg-ink-700"
  >
    <span
        class="absolute inset-y-0 left-0 w-[2px] origin-top scale-y-0 bg-acid transition-transform duration-700 ease-deck group-hover:scale-y-100"
        aria-hidden="true"
    />

    <div class="p-4 pl-5">
      <div class="flex items-center gap-3">
        <span class="font-mono text-[9px] uppercase tracking-[0.28em] text-acid">Подборка</span>
        <span class="h-px w-6 bg-acid/40"/>
        <span class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">{{ packId }}</span>
      </div>

      <h3 class="mt-2.5 font-unbounded text-[16px] font-semibold leading-[1.1] tracking-[-0.045em] text-fg">
        {{ pack.name }}
      </h3>
      <p class="mt-2 line-clamp-2 max-w-[42ch] text-[12px] leading-relaxed text-fg-muted">
        {{ pack.description }}
      </p>
    </div>

    <div class="flex items-center justify-between border-t border-line px-4 py-2 pl-5">
      <div class="flex items-center gap-3 font-mono text-[9px] uppercase tracking-[0.14em] text-fg-faint">
        <span>{{ pack.minecraftVersion }}</span>
        <span class="h-3 w-px bg-line"/>
        <span>{{ pack.type }}</span>
        <span class="h-3 w-px bg-line"/>
        <span>rev {{ pack.version }}</span>
      </div>

      <AppButton tone="quiet" class="group/act text-[10px] tracking-[0.18em]" @click="emit('install', packId)">
        Загрузить
        <UIcon
            name="i-lucide-arrow-right"
            class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-1"
        />
      </AppButton>
    </div>
  </article>
</template>
