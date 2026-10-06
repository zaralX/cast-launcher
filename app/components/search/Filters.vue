<script setup lang="ts">
import type { PackCapabilities, PackCategory, PackFilters, PackEnvironment } from '~/types/catalog'

const props = defineProps<{
  filters: PackFilters | null
  capabilities?: PackCapabilities | null
  loading?: boolean
}>()

const can = computed<PackCapabilities>(() => props.capabilities ?? {
  multipleGameVersions: true,
  environment: true,
  blockableFiles: false,
})

const loaders = defineModel<string[]>('loaders', { required: true })
const gameVersions = defineModel<string[]>('gameVersions', { required: true })
const categories = defineModel<string[]>('categories', { required: true })
const environment = defineModel<PackEnvironment | null>('environment', { required: true })

const { t } = useI18n()

const ANY_ENVIRONMENT = 'any'

const ENVIRONMENTS = computed<{ value: PackEnvironment | typeof ANY_ENVIRONMENT, label: string }[]>(() => [
  { value: ANY_ENVIRONMENT, label: t('search.filters.env_any') },
  { value: 'client', label: t('search.filters.env_client') },
  { value: 'server', label: t('search.filters.env_server') },
])

const environmentChoice = computed({
  get: () => environment.value ?? ANY_ENVIRONMENT,
  set: (value: PackEnvironment | typeof ANY_ENVIRONMENT) => {
    environment.value = value === ANY_ENVIRONMENT ? null : value
  },
})

const groups = computed(() => {
  const byHeader = new Map<string, PackCategory[]>()

  for (const category of props.filters?.categories ?? []) {
    const header = category.header || 'categories'
    byHeader.set(header, [...(byHeader.get(header) ?? []), category])
  }

  return [...byHeader.entries()].map(([header, items]) => ({ header, items }))
})

const ANY_VERSION = 'any'

const gameVersionItems = computed(() => [
  { label: t('search.filters.any_version'), value: ANY_VERSION },
  ...(props.filters?.gameVersions ?? []).map(version => ({ label: version, value: version })),
])

const singleGameVersion = computed({
  get: () => gameVersions.value[0] ?? ANY_VERSION,
  set: (value: string) => {
    gameVersions.value = value && value !== ANY_VERSION ? [value] : []
  },
})

const selectedCount = computed(() =>
  loaders.value.length + gameVersions.value.length + categories.value.length + (environment.value ? 1 : 0),
)

const reset = () => {
  loaders.value = []
  gameVersions.value = []
  categories.value = []
  environment.value = null
}

const HEADER_KEYS: Record<string, string> = {
  'categories': 'search.filters.group.categories',
  'resolutions': 'search.filters.group.resolutions',
  'performance impact': 'search.filters.group.performance',
}
</script>

<template>
  <aside class="flex flex-col gap-7">
    <div class="flex items-center gap-3">
      <span class="font-mono text-label uppercase tracking-caps-wide text-acid">{{ $t('search.filters.title') }}</span>
      <Separator class="flex-1" />
      <Button
        v-if="selectedCount"
        variant="quiet"
        class="py-0 text-micro"
        @click="reset"
      >
        {{ $t('search.filters.reset', { count: selectedCount }) }}
      </Button>
    </div>

    <div
      v-if="loading"
      class="space-y-3"
    >
      <Skeleton
        v-for="row in 4"
        :key="row"
        class="h-7"
      />
    </div>

    <template v-else-if="filters">
      <section v-if="filters.loaders.length">
        <Label class="mb-3 text-micro">
          {{ $t('search.filters.loader') }}
        </Label>
        <ToggleGroup
          v-model="loaders"
          type="multiple"
        >
          <ToggleGroupItem
            v-for="loader in filters.loaders"
            :key="loader"
            :value="loader"
          >
            {{ loader }}
          </ToggleGroupItem>
        </ToggleGroup>
      </section>

      <section v-if="filters.gameVersions.length">
        <Label class="mb-3 text-micro">
          {{ $t('search.filters.game_version') }}
        </Label>
        <KitSelectMenu
          v-if="can.multipleGameVersions"
          v-model="gameVersions"
          :items="filters.gameVersions"
          multiple
          :placeholder="$t('search.filters.any_version')"
        />
        <template v-else>
          <KitSelectMenu
            v-model="singleGameVersion"
            :items="gameVersionItems"
          />
          <p class="mt-2 text-caption leading-relaxed text-fg-faint">
            {{ $t('search.filters.single_version') }}
          </p>
        </template>
      </section>

      <section v-if="can.environment">
        <Label class="mb-3 text-micro">
          {{ $t('search.filters.environment') }}
        </Label>
        <RadioGroup v-model="environmentChoice">
          <RadioGroupItem
            v-for="option in ENVIRONMENTS"
            :key="option.value"
            :value="option.value"
            class="py-2 font-mono text-micro uppercase tracking-caps data-[state=checked]:text-acid"
          >
            {{ option.label }}
          </RadioGroupItem>
        </RadioGroup>
      </section>

      <section
        v-for="group in groups"
        :key="group.header"
      >
        <Label class="mb-3 text-micro">
          {{ HEADER_KEYS[group.header] ? $t(HEADER_KEYS[group.header]!) : group.header }}
        </Label>
        <ToggleGroup
          v-model="categories"
          type="multiple"
        >
          <ToggleGroupItem
            v-for="category in group.items"
            :key="category.id"
            :value="category.id"
          >
            {{ categoryLabel(category) }}
          </ToggleGroupItem>
        </ToggleGroup>
      </section>
    </template>

    <p
      v-else
      class="text-body leading-relaxed text-fg-muted"
    >
      {{ $t('search.filters.failed') }}
    </p>
  </aside>
</template>
