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
  <article
    class="relative flex items-start gap-4 rounded-md p-3.5 transition-colors duration-150 before:absolute before:inset-x-3.5 before:top-0 before:h-px before:bg-line first:before:hidden hover:bg-panel hover:before:opacity-0 [&:hover+article]:before:opacity-0"
  >
    <KitThumb
      :src="hit.iconUrl"
      :alt="hit.title"
      size="lg"
    />

    <div class="min-w-0 flex-1">
      <div class="flex min-w-0 items-baseline gap-2">
        <h3 class="truncate text-title font-semibold text-fg">
          {{ hit.title }}
        </h3>
        <span
          v-if="hit.author"
          class="shrink-0 text-caption text-fg-faint"
        >{{ hit.author }}</span>
      </div>

      <p class="mt-0.5 line-clamp-2 text-body text-fg-muted">
        {{ hit.description }}
      </p>

      <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1.5 text-caption text-fg-faint">
        <span class="flex items-center gap-1.5">
          <Icon
            name="i-lucide-download"
            class="size-3.5"
          />
          {{ formatDownloads(hit.downloads) }}
        </span>

        <span
          v-if="latestVersion"
          class="font-mono"
        >{{ latestVersion }}</span>

        <template v-if="categories.length">
          <Separator
            orientation="vertical"
            class="data-[orientation=vertical]:h-3"
          />

          <span class="flex flex-wrap gap-1">
            <Badge
              v-for="category in categories"
              :key="category"
            >
              {{ categoryName(category) }}
            </Badge>
          </span>
        </template>
      </div>
    </div>

    <Button
      class="mt-3"
      @click="emit('install', hit)"
    >
      {{ $t('search.card.install') }}
    </Button>
  </article>
</template>
