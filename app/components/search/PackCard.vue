<script setup lang="ts">
import type {PackHit} from "~/types/catalog";
import {categoryName, formatDownloads} from "~/types/catalog";

const props = defineProps<{ hit: PackHit }>()

const emit = defineEmits<{ install: [hit: PackHit] }>()

const categories = computed(() => {
  const list = props.hit.displayCategories.length ? props.hit.displayCategories : props.hit.categories
  return list.slice(0, 4)
})

const latestVersion = computed(() => props.hit.versions.at(-1) ?? "")
</script>

<template>
  <article
      class="group relative flex gap-4 overflow-hidden border border-line bg-ink-800 p-4 transition-all duration-500 ease-deck hover:border-acid/40 hover:bg-ink-700"
  >
    <span
        class="absolute inset-y-0 left-0 w-[2px] origin-top scale-y-0 bg-acid transition-transform duration-700 ease-deck group-hover:scale-y-100"
        aria-hidden="true"
    />

    <span class="grid size-14 shrink-0 place-items-center overflow-hidden border border-line bg-ink-900">
      <img
          v-if="hit.iconUrl"
          :src="hit.iconUrl"
          :alt="hit.title"
          loading="lazy"
          class="size-full object-cover"
      />
      <UIcon v-else name="i-lucide-package" class="size-5 text-fg-faint"/>
    </span>

    <div class="flex min-w-0 flex-1 flex-col">
      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <h3 class="truncate font-unbounded text-[14px] font-semibold leading-[1.15] tracking-[-0.04em] text-fg">
            {{ hit.title }}
          </h3>
          <p v-if="hit.author" class="mt-1 font-mono text-[9px] uppercase tracking-[0.16em] text-fg-faint">
            {{ hit.author }}
          </p>
        </div>

        <AppButton
            tone="quiet"
            class="group/act shrink-0 text-[10px] tracking-[0.18em]"
            @click="emit('install', hit)"
        >
          Установить
          <UIcon
              name="i-lucide-arrow-right"
              class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-1"
          />
        </AppButton>
      </div>

      <p class="mt-2 line-clamp-2 text-[12px] leading-relaxed text-fg-muted">
        {{ hit.description }}
      </p>

      <div class="mt-auto flex flex-wrap items-center gap-x-3 gap-y-2 pt-3">
        <span class="flex items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.14em] text-fg-faint">
          <UIcon name="i-lucide-download" class="size-3"/>
          {{ formatDownloads(hit.downloads) }}
        </span>

        <span v-if="latestVersion" class="font-mono text-[9px] uppercase tracking-[0.14em] text-fg-faint">
          {{ latestVersion }}
        </span>

        <span class="h-3 w-px bg-line"/>

        <span
            v-for="category in categories"
            :key="category"
            class="border border-line px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.12em] text-fg-faint"
        >
          {{ categoryName(category) }}
        </span>
      </div>
    </div>
  </article>
</template>
