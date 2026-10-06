<script setup lang="ts">
import type { PackHit } from '~/types/catalog'

const props = defineProps<{ hit: PackHit }>()

const emit = defineEmits<{ install: [hit: PackHit] }>()

const categories = computed(() => {
  const list = props.hit.displayCategories.length ? props.hit.displayCategories : props.hit.categories
  return list.slice(0, 4)
})

const latestVersion = computed(() => props.hit.versions.at(-1) ?? '')
</script>

<template>
  <article class="group relative flex gap-4 overflow-hidden border border-line bg-ink-800 p-4 transition-all duration-500 ease-deck hover:border-acid/40 hover:bg-ink-700">
    <span
      class="absolute inset-y-0 left-0 w-[2px] origin-top scale-y-0 bg-acid transition-transform duration-700 ease-deck group-hover:scale-y-100"
      aria-hidden="true"
    />

    <KitThumb
      :src="hit.iconUrl"
      :alt="hit.title"
      size="lg"
    />

    <div class="flex min-w-0 flex-1 flex-col">
      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <h3 class="truncate font-unbounded text-lead font-semibold leading-[1.15] tracking-heading text-fg">
            {{ hit.title }}
          </h3>
          <p
            v-if="hit.author"
            class="mt-1 font-mono text-micro uppercase tracking-caps text-fg-faint"
          >
            {{ hit.author }}
          </p>
        </div>

        <Button
          variant="quiet"
          class="group/act shrink-0"
          @click="emit('install', hit)"
        >
          {{ $t('search.card.install') }}
          <Icon
            name="i-lucide-arrow-right"
            class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-1"
          />
        </Button>
      </div>

      <p class="mt-2 line-clamp-2 text-body leading-relaxed text-fg-muted">
        {{ hit.description }}
      </p>

      <div class="mt-auto flex flex-wrap items-center gap-x-3 gap-y-2 pt-3">
        <span class="flex items-center gap-1.5 font-mono text-micro uppercase tracking-caps text-fg-faint">
          <Icon
            name="i-lucide-download"
            class="size-3"
          />
          {{ formatDownloads(hit.downloads) }}
        </span>

        <span
          v-if="latestVersion"
          class="font-mono text-micro uppercase tracking-caps text-fg-faint"
        >
          {{ latestVersion }}
        </span>

        <Separator
          orientation="vertical"
          class="h-3"
        />

        <Badge
          v-for="category in categories"
          :key="category"
          class="text-fg-faint"
        >
          {{ categoryName(category) }}
        </Badge>
      </div>
    </div>
  </article>
</template>
