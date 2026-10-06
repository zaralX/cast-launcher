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
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <RadioGroup v-model="tab">
        <RadioGroupItem
          v-for="item in TABS"
          :key="item.key"
          :value="item.key"
          class="flex-none px-4 py-2 font-mono text-label uppercase tracking-caps"
        >
          {{ $t(item.labelKey) }}
        </RadioGroupItem>
      </RadioGroup>

      <div class="flex items-center gap-3">
        <Button
          icon="i-lucide-image-plus"
          :loading="importing"
          @click="importFile"
        >
          {{ $t('icon.upload') }}
        </Button>

        <Button
          variant="quiet"
          icon="i-lucide-x"
          :disabled="!icon"
          @click="icon = ''"
        >
          {{ $t('icon.none') }}
        </Button>
      </div>
    </div>

    <div v-if="tab === 'library'">
      <div
        v-if="library.length"
        class="grid max-h-[22rem] grid-cols-8 gap-2 overflow-y-auto pr-1"
      >
        <button
          v-for="file in library"
          :key="file.name"
          type="button"
          class="group relative grid aspect-square cursor-pointer place-items-center border outline-none transition-colors duration-300 focus-visible:border-line-strong"
          :class="icon === file.name ? 'border-acid bg-ink-700' : 'border-line hover:border-line-strong hover:bg-ink-700'"
          :title="file.name"
          @click="icon = file.name"
        >
          <InstanceIcon
            :icon="file.name"
            size="md"
            :bordered="false"
          />

          <span
            class="absolute -top-px -right-px hidden size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 group-hover:grid hover:border-danger/50 hover:text-danger"
            :title="$t('icon.remove', { name: file.name })"
            @click.stop="removeIcon(file.name)"
          >
            <Spinner
              v-if="removing === file.name"
              class="size-3"
            />
            <Icon
              v-else
              name="i-lucide-x"
              class="size-3"
            />
          </span>
        </button>
      </div>

      <KitEmpty v-else>
        {{ $t('icon.library_empty') }}
      </KitEmpty>
    </div>

    <div
      v-else
      class="space-y-4"
    >
      <div class="flex flex-wrap items-end gap-4">
        <KitField
          :label="$t('icon.category')"
          class="min-w-52 flex-1"
        >
          <KitSelect
            v-model="category"
            :items="categoryItems"
          />
        </KitField>

        <KitField
          :label="$t('icon.search')"
          class="min-w-44 flex-1"
        >
          <KitSearchInput
            v-model="search"
            :placeholder="$t('icon.search_placeholder')"
          />
        </KitField>
      </div>

      <KitLoading
        v-if="catalogLoading && !catalog"
        :label="$t('icon.catalog_loading')"
        class="py-12"
      />

      <template v-else>
        <div
          v-if="visibleItems.length"
          class="grid max-h-[19rem] grid-cols-8 gap-2 overflow-y-auto pr-1"
        >
          <button
            v-for="item in visibleItems"
            :key="item"
            type="button"
            class="grid aspect-square cursor-pointer place-items-center border border-line outline-none transition-colors duration-300 hover:border-acid/50 hover:bg-ink-700 focus-visible:border-acid/50"
            :title="itemName(item)"
            @click="useItem(item)"
          >
            <Spinner
              v-if="saving === item"
              class="text-acid"
            />
            <img
              v-else-if="iconStore.itemUrlOf(item)"
              :src="iconStore.itemUrlOf(item)!"
              :alt="itemName(item)"
              class="size-8 object-contain [image-rendering:pixelated]"
            >
            <span
              v-else
              class="size-4 bg-line/60"
            />
          </button>
        </div>

        <KitEmpty v-else>
          {{ $t('icon.nothing_found') }}
        </KitEmpty>

        <div class="flex items-center justify-between gap-4">
          <p class="font-mono text-label uppercase tracking-caps text-fg-faint">
            {{ $t('icon.shown', { shown: visibleItems.length, total: catalogItems.length }) }}
          </p>

          <Button
            v-if="visibleItems.length < catalogItems.length"
            variant="quiet"
            icon="i-lucide-chevron-down"
            @click="limit += PAGE"
          >
            {{ $t('icon.show_more') }}
          </Button>
        </div>
      </template>
    </div>
  </div>
</template>
