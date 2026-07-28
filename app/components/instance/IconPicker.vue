<script setup lang="ts">
import {storeToRefs} from "pinia";
import {itemCategoryLabel, itemFallbackName} from "~/types/icon";

const icon = defineModel<string>({required: true})

const iconStore = useIconStore()
const {library, catalog, catalogLoading} = storeToRefs(iconStore)

const PAGE = 96

const ALL_CATEGORIES = "all"

type Tab = "library" | "catalog"

const tab = ref<Tab>("library")
const importing = ref(false)
const saving = ref("")
const removing = ref("")
const search = ref("")
const category = ref(ALL_CATEGORIES)
const limit = ref(PAGE)

const categories = computed(() => Object.keys(catalog.value?.categories ?? {}))

const categoryItems = computed(() => [
  {label: "Все категории", value: ALL_CATEGORIES},
  ...categories.value.map(key => ({label: itemCategoryLabel(key), value: key}))
])

const itemName = (item: string) => catalog.value?.names?.[item] ?? itemFallbackName(item)

const catalogItems = computed(() => {
  const groups = catalog.value?.categories ?? {}
  const picked = category.value === ALL_CATEGORIES
      ? Object.values(groups)
      : [groups[category.value] ?? []]
  const items = picked.flat()
  const needle = search.value.trim().toLowerCase()

  if (!needle) return items

  return items.filter(item => item.includes(needle) || itemName(item).toLowerCase().includes(needle))
})

const visibleItems = computed(() => catalogItems.value.slice(0, limit.value))

watch([search, category], () => limit.value = PAGE)

watch(visibleItems, (items) => {
  if (items.length) safeRun(() => iconStore.ensureItemUrls(items))
}, {immediate: true})

watch(tab, async (value) => {
  if (value !== "catalog" || catalog.value) return

  await safeRun(() => iconStore.loadCatalog(), {code: "NETWORK", context: {action: "Каталог иконок"}})
})

async function importFile() {
  if (importing.value) return
  importing.value = true

  const result = await attempt(() => iconStore.importFile(), {context: {action: "Загрузка иконки"}})

  importing.value = false

  if (result.ok && result.value) {
    icon.value = result.value.name
    tab.value = "library"
  }
}

async function useItem(item: string) {
  if (saving.value) return
  saving.value = item

  const result = await attempt(() => iconStore.useItem(item), {context: {action: "Сохранение иконки"}})

  saving.value = ""

  if (result.ok) icon.value = result.value.name
}

async function removeIcon(name: string) {
  if (removing.value) return
  removing.value = name

  const result = await attempt(() => iconStore.removeIcon(name), {context: {action: "Удаление иконки"}})

  removing.value = ""

  if (result.ok && icon.value === name) icon.value = ""
}

onMounted(async () => {
  await safeRun(() => iconStore.loadLibrary(), {context: {action: "Библиотека иконок"}})
  library.value.forEach(file => iconStore.ensureUrl(file.name))
})
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex border border-line">
        <button
            v-for="(item, i) in [{key: 'library', label: 'Библиотека'}, {key: 'catalog', label: 'Каталог Minecraft'}]"
            :key="item.key"
            type="button"
            class="relative px-4 py-2 font-mono text-[10px] uppercase tracking-[0.2em] transition-colors duration-300"
            :class="[
              i > 0 ? 'border-l border-line' : '',
              tab === item.key ? 'bg-ink-700 text-fg' : 'text-fg-faint hover:text-fg-muted'
            ]"
            @click="tab = item.key as Tab"
        >
          <span
              class="absolute inset-x-0 top-0 h-px origin-center bg-acid transition-transform duration-500 ease-deck"
              :class="tab === item.key ? 'scale-x-100' : 'scale-x-0'"
          />
          {{ item.label }}
        </button>
      </div>

      <div class="flex items-center gap-3">
        <AppButton
            class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
            icon="i-lucide-image-plus"
            :loading="importing"
            @click="importFile"
        >
          Загрузить файл
        </AppButton>

        <AppButton
            tone="quiet"
            class="text-[10px] tracking-[0.18em]"
            icon="i-lucide-x"
            :disabled="!icon"
            @click="icon = ''"
        >
          Без иконки
        </AppButton>
      </div>
    </div>

    <div v-if="tab === 'library'">
      <div v-if="library.length" class="grid max-h-[22rem] grid-cols-6 gap-2 overflow-y-auto pr-1 sm:grid-cols-8">
        <button
            v-for="file in library"
            :key="file.name"
            type="button"
            class="group relative grid aspect-square place-items-center border transition-colors duration-300"
            :class="icon === file.name ? 'border-acid bg-ink-700' : 'border-line hover:border-line-strong hover:bg-ink-700'"
            :title="file.name"
            @click="icon = file.name"
        >
          <InstanceIcon :icon="file.name" size="md" :bordered="false"/>

          <span
              class="absolute -right-px -top-px hidden size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 hover:border-red-400/50 hover:text-red-400 group-hover:grid"
              :title="`Удалить ${file.name}`"
              @click.stop="removeIcon(file.name)"
          >
            <UIcon
                :name="removing === file.name ? 'i-lucide-loader-circle' : 'i-lucide-x'"
                class="size-3"
                :class="removing === file.name ? 'animate-spin' : ''"
            />
          </span>
        </button>
      </div>

      <p v-else class="border border-dashed border-line py-12 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
        Библиотека пуста — загрузите файл или выберите иконку из каталога
      </p>
    </div>

    <div v-else class="space-y-4">
      <div class="flex flex-wrap items-end gap-4">
        <SettingsField label="Категория" class="min-w-[13rem] flex-1">
          <USelect v-model="category" :items="categoryItems" class="w-full"/>
        </SettingsField>

        <SettingsField label="Поиск" class="min-w-[11rem] flex-1">
          <UInput v-model="search" placeholder="Например, меч" class="w-full">
            <template #trailing>
              <UIcon name="i-lucide-search" class="size-3.5 text-fg-faint"/>
            </template>
          </UInput>
        </SettingsField>
      </div>

      <div v-if="catalogLoading && !catalog" class="flex flex-col items-center gap-4 py-12">
        <span class="relative block h-px w-40 overflow-hidden bg-line">
          <span class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"/>
        </span>
        <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Загрузка каталога</p>
      </div>

      <template v-else>
        <div v-if="visibleItems.length" class="grid max-h-[19rem] grid-cols-6 gap-2 overflow-y-auto pr-1 sm:grid-cols-8">
          <button
              v-for="item in visibleItems"
              :key="item"
              type="button"
              class="grid aspect-square place-items-center border border-line transition-colors duration-300 hover:border-acid/50 hover:bg-ink-700"
              :title="itemName(item)"
              @click="useItem(item)"
          >
            <UIcon
                v-if="saving === item"
                name="i-lucide-loader-circle"
                class="size-4 animate-spin text-acid"
            />
            <img
                v-else-if="iconStore.itemUrlOf(item)"
                :src="iconStore.itemUrlOf(item)!"
                :alt="itemName(item)"
                class="size-8 object-contain [image-rendering:pixelated]"
            />
            <span v-else class="size-4 bg-line/60"/>
          </button>
        </div>

        <p v-else class="border border-dashed border-line py-12 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
          Ничего не найдено
        </p>

        <div class="flex items-center justify-between gap-4">
          <p class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
            {{ visibleItems.length }} из {{ catalogItems.length }}
          </p>

          <AppButton
              v-if="visibleItems.length < catalogItems.length"
              tone="quiet"
              class="text-[10px] tracking-[0.18em]"
              icon="i-lucide-chevron-down"
              @click="limit += PAGE"
          >
            Показать ещё
          </AppButton>
        </div>
      </template>
    </div>

    <p class="border-t border-line pt-4 font-mono text-[10px] leading-relaxed text-fg-faint/70">
      Иконки предметов берутся с ассетов zaralX и сохраняются в библиотеку лаунчера — дальше они работают без сети.
    </p>
  </div>
</template>
