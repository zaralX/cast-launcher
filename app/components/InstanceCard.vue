<script setup lang="ts">
import type {Instance} from "~/types/instance";
import {formatPlaytime} from "~/types/instance";

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

const state = computed<"running" | "installing" | "ready" | "absent">(() => {
  if (props.running) return "running"
  if (props.installing) return "installing"
  return props.instance.installed ? "ready" : "absent"
})

const playtime = computed(() => formatPlaytime(props.instance.playtime?.totalSeconds ?? 0) || "не запускалась")
</script>

<template>
  <article
      class="cut-16 group relative flex flex-col border border-line bg-ink-800 p-3.5 transition-all duration-500 ease-deck hover:-translate-y-0.5 hover:border-acid/40 hover:bg-ink-700"
  >
    <span
        class="pointer-events-none absolute right-0 top-0 h-[23px] w-px origin-top-right rotate-45 bg-line transition-colors duration-500 group-hover:bg-acid/40"
        aria-hidden="true"
    />

    <header class="flex min-w-0 items-center gap-3">
      <InstanceIcon
          :icon="instance.icon"
          :type="instance.type"
          class="text-fg-faint transition-colors duration-500 group-hover:border-acid/40 group-hover:text-acid"
      />

      <div class="min-w-0 flex-1 pr-3">
        <h3
            class="truncate font-unbounded text-[13px] font-semibold leading-tight tracking-[-0.035em] text-fg"
            :title="instance.name"
        >
          {{ instance.name }}
        </h3>
        <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.16em] text-fg-faint">
          {{ instance.minecraftVersion }}
          <template v-if="instance.loaderVersion"> · {{ instance.loaderVersion }}</template>
        </p>
        <p
            class="mt-1 flex items-center gap-1.5 truncate font-mono text-[9px] uppercase tracking-[0.16em] text-fg-faint"
            :title="`Наиграно: ${playtime}`"
        >
          <UIcon name="i-lucide-timer" class="size-2.5 shrink-0"/>
          <span class="truncate">{{ playtime }}</span>
        </p>
      </div>
    </header>

    <div class="mt-3 h-px w-full bg-line transition-colors duration-500 group-hover:bg-line-strong"/>

    <footer class="mt-2.5 flex items-center gap-2">
      <div class="min-w-0 flex-1">
        <div v-if="state === 'running'" class="flex h-8 items-center gap-2.5 px-1">
          <span class="relative grid size-2 place-items-center">
            <span class="absolute size-2 bg-acid animate-breathe"/>
            <span class="size-2 bg-acid"/>
          </span>
          <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-acid">Запущено</span>
        </div>

        <div v-else-if="state === 'installing'" class="flex h-8 flex-col justify-center gap-1.5 px-1">
          <span class="flex items-baseline justify-between gap-3">
            <span class="min-w-0 truncate font-mono text-[9px] uppercase tracking-[0.18em] text-fg-muted">
              {{ phase || 'Установка' }}
            </span>
            <span v-if="progress != null" class="shrink-0 font-mono text-[9px] tabular-nums text-acid">
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
            class="group/act h-8 text-[10px] tracking-[0.18em]"
            @click="state === 'ready' ? emit('run', instance.id) : emit('install', instance.id)"
        >
          <template #leading>
            <UIcon
                :name="state === 'ready' ? 'i-lucide-play' : 'i-lucide-arrow-down-to-line'"
                class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-0.5"
            />
          </template>
          {{ state === 'ready' ? 'Играть' : 'Загрузить' }}
        </AppButton>
      </div>

      <NuxtLink
          :to="`/instance/${instance.id}`"
          :aria-label="`Настройки сборки ${instance.name}`"
          :title="`Настройки сборки ${instance.name}`"
          class="grid size-8 shrink-0 place-items-center border border-line text-fg-faint transition-colors duration-300 hover:border-acid hover:text-acid"
      >
        <UIcon name="i-lucide-settings" class="size-3.5 transition-transform duration-500 ease-deck hover:rotate-90"/>
      </NuxtLink>
    </footer>
  </article>
</template>
