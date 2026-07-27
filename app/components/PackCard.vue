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
    <!-- Кислотный кант слева: раскрывается сверху вниз при наведении -->
    <span
        class="absolute inset-y-0 left-0 w-[2px] origin-top scale-y-0 bg-acid transition-transform duration-700 ease-deck group-hover:scale-y-100"
        aria-hidden="true"
    />

    <div class="p-6 pl-7">
      <div class="flex items-center gap-3">
        <span class="font-mono text-[9px] uppercase tracking-[0.28em] text-acid">Подборка</span>
        <span class="h-px w-6 bg-acid/40"/>
        <span class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">{{ packId }}</span>
      </div>

      <h3 class="mt-4 font-unbounded text-[22px] font-semibold leading-[1.05] tracking-[-0.045em] text-fg">
        {{ pack.name }}
      </h3>
      <p class="mt-3 max-w-[34ch] text-[13px] leading-relaxed text-fg-muted">
        {{ pack.description }}
      </p>
    </div>

    <div class="flex items-center justify-between border-t border-line px-6 py-3 pl-7">
      <div class="flex items-center gap-4 font-mono text-[10px] uppercase tracking-[0.14em] text-fg-faint">
        <span>{{ pack.minecraftVersion }}</span>
        <span class="h-3 w-px bg-line"/>
        <span>{{ pack.type }}</span>
        <span class="h-3 w-px bg-line"/>
        <span>rev {{ pack.version }}</span>
      </div>

      <button
          type="button"
          class="group/act relative -mr-2 flex items-center gap-2 px-2 py-1 font-mono text-[11px] uppercase tracking-[0.18em] text-fg-muted transition-colors duration-300 hover:text-acid"
          @click="emit('install', packId)"
      >
        Загрузить
        <UIcon
            name="i-lucide-arrow-right"
            class="size-3.5 transition-transform duration-500 ease-deck group-hover/act:translate-x-1"
        />
      </button>
    </div>
  </article>
</template>
