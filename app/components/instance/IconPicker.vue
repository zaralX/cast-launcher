<script setup lang="ts">
const icon = defineModel<string>({ required: true })

const { t } = useI18n()
const iconStore = useIconStore()
const { library, catalog, catalogLoading } = storeToRefs(iconStore)

const PAGE = 96

const ALL_CATEGORIES = 'all'

type Tab = 'library' | 'catalog'

const TABS: { key: Tab, labelKey: string }[] = [
  { key: 'library', labelKey: 'icon.tab.library' },
  { key: 'catalog', labelKey: 'icon.tab.catalog' },
]

const tab = ref<Tab>('library')
const importing = ref(false)
const saving = ref('')
const removing = ref('')
const search = ref('')
const category = ref(ALL_CATEGORIES)
const limit = ref(PAGE)

const categories = computed(() => Object.keys(catalog.value?.categories ?? {}))

const categoryItems = computed(() => [
  { label: t('icon.all_categories'), value: ALL_CATEGORIES },
  ...categories.value.map(key => ({ label: itemCategoryLabel(key), value: key })),
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
}, { immediate: true })

watch(tab, async (value) => {
  if (value !== 'catalog' || catalog.value) return

  await safeRun(() => iconStore.loadCatalog(), { code: 'NETWORK', context: { action: t('icon.catalog_action') } })
})

async function importFile() {
  if (importing.value) return
  importing.value = true

  const result = await attempt(() => iconStore.importFile(), { context: { action: t('icon.upload_action') } })

  importing.value = false

  if (result.ok && result.value) {
    icon.value = result.value.name
    tab.value = 'library'
  }
}

async function useItem(item: string) {
  if (saving.value) return
  saving.value = item

  const result = await attempt(() => iconStore.useItem(item), { context: { action: t('icon.save_action') } })

  saving.value = ''

  if (result.ok) icon.value = result.value.name
}

async function removeIcon(name: string) {
  if (removing.value) return
  removing.value = name

  const result = await attempt(() => iconStore.removeIcon(name), { context: { action: t('icon.remove_action') } })

  removing.value = ''

  if (result.ok && icon.value === name) icon.value = ''
}

onMounted(async () => {
  await safeRun(() => iconStore.loadLibrary(), { context: { action: t('icon.library_action') } })
  library.value.forEach(file => iconStore.ensureUrl(file.name))
})
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center gap-2">
      <RadioGroup v-model="tab">
        <RadioGroupItem
          v-for="item in TABS"
          :key="item.key"
          :value="item.key"
          class="h-7.5 flex-none px-3"
        >
          {{ $t(item.labelKey) }}
        </RadioGroupItem>
      </RadioGroup>

      <span class="flex-1" />

      <Button
        variant="ghost"
        icon="i-lucide-x"
        class="-mr-2"
        :disabled="!icon"
        @click="icon = ''"
      >
        {{ $t('icon.none') }}
      </Button>
    </div>

    <div
      v-if="tab === 'library'"
      class="grid max-h-[22rem] grid-cols-6 gap-2.5 overflow-y-auto p-0.5"
    >
      <div
        v-for="file in library"
        :key="file.name"
        class="group/icon relative"
      >
        <KitTile
          :selected="icon === file.name"
          class="grid aspect-square w-full place-items-center"
          :title="file.name"
          @click="icon = file.name"
        >
          <InstanceIcon
            :icon="file.name"
            size="lg"
          />
        </KitTile>

        <Button
          size="icon-sm"
          icon="i-lucide-x"
          :loading="removing === file.name"
          :aria-label="$t('icon.remove', { name: file.name })"
          :title="$t('icon.remove', { name: file.name })"
          class="absolute top-1.5 right-1.5 bg-raise opacity-0 shadow-float ring-1 ring-line-strong group-hover/icon:opacity-100 focus-visible:opacity-100"
          @click="removeIcon(file.name)"
        />
      </div>

      <Button
        variant="dashed"
        icon="i-lucide-image-plus"
        :loading="importing"
        :aria-label="$t('icon.upload')"
        :title="$t('icon.upload')"
        class="aspect-square h-auto rounded-md"
        @click="importFile"
      />
    </div>

    <div
      v-else
      class="space-y-4"
    >
      <div class="flex items-center gap-2">
        <KitSelect
          v-model="category"
          :items="categoryItems"
          :aria-label="$t('icon.category')"
          class="w-56"
        />
        <KitSearchInput
          v-model="search"
          :placeholder="$t('icon.search')"
          :aria-label="$t('icon.search')"
          class="min-w-0 flex-1"
        />
      </div>

      <KitLoading
        v-if="catalogLoading && !catalog"
        :label="$t('icon.catalog_loading')"
        class="py-12"
      />

      <template v-else>
        <div
          v-if="visibleItems.length"
          class="grid max-h-[19rem] grid-cols-8 gap-2 overflow-y-auto p-0.5"
        >
          <KitTile
            v-for="item in visibleItems"
            :key="item"
            class="grid aspect-square place-items-center"
            :title="itemName(item)"
            @click="useItem(item)"
          >
            <Spinner
              v-if="saving === item"
              class="text-accent"
            />
            <img
              v-else-if="iconStore.itemUrlOf(item)"
              :src="iconStore.itemUrlOf(item)!"
              :alt="itemName(item)"
              class="size-8 object-contain [image-rendering:pixelated]"
            >
            <span
              v-else
              class="size-4 rounded-xs bg-control"
            />
          </KitTile>
        </div>

        <KitEmpty
          v-else
          icon="i-lucide-search-x"
        >
          {{ $t('icon.nothing_found') }}
        </KitEmpty>

        <div class="flex items-center justify-between gap-4">
          <p class="text-caption text-fg-faint tabular-nums">
            {{ $t('icon.shown', { shown: visibleItems.length, total: catalogItems.length }) }}
          </p>
          <Button
            v-if="visibleItems.length < catalogItems.length"
            variant="ghost"
            size="sm"
            icon="i-lucide-chevron-down"
            class="-mr-2.5"
            @click="limit += PAGE"
          >
            {{ $t('icon.show_more') }}
          </Button>
        </div>
      </template>
    </div>
  </div>
</template>
