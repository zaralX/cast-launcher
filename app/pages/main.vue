<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { useLocalStorage } from '@vueuse/core'

definePageMeta({
  layout: 'main',
})

type Sort = 'recent' | 'name' | 'playtime'

const SORTS: Sort[] = ['recent', 'name', 'playtime']

const { t, locale } = useI18n()
const appStore = useAppStore()
const instanceStore = useInstanceStore()
const toast = useAppToast()

const { instances } = storeToRefs(instanceStore)

const createModalOpen = ref(false)
const importModalOpen = ref(false)
const launcherImportOpen = ref(false)
const announceImport = useImportToast()

const compact = useCompact()

const view = computed({
  get: () => (compact.value ? 'compact' : 'normal'),
  set: (value: string) => setCompact(value === 'compact'),
})

async function setCompact(value: boolean) {
  const config = appStore.config
  if (!config || config.launcher.compact === value) return

  await safeRun(
    () => appStore.updateConfig({ ...config, launcher: { ...config.launcher, compact: value } }),
    { context: { action: t('home.view.label') } },
  )
}

const query = ref('')
const sort = useLocalStorage<Sort>('cast:library-sort', 'recent')
const sortLabel = computed(() => t(`home.sort.${SORTS.includes(sort.value) ? sort.value : 'recent'}`))

const shown = computed(() => {
  const needle = query.value.trim().toLocaleLowerCase(locale.value)
  const list = instances.value.filter(instance => !needle
    || instance.name.toLocaleLowerCase(locale.value).includes(needle)
    || instance.minecraftVersion.includes(needle))

  switch (sort.value) {
    case 'name':
      return list.sort((a, b) => a.name.localeCompare(b.name, locale.value))
    case 'playtime':
      return list.sort((a, b) => (b.playtime?.totalSeconds ?? 0) - (a.playtime?.totalSeconds ?? 0))
    default:
      return list.sort((a, b) => (b.playtime?.lastPlayedAt ?? 0) - (a.playtime?.lastPlayedAt ?? 0))
  }
})

const selectedId = ref<string | null>(null)
const selected = computed(() => shown.value.find(instance => instance.id === selectedId.value) ?? null)

watch([compact, shown], () => {
  if (compact.value && !selected.value) selectedId.value = shown.value[0]?.id ?? null
}, { immediate: true })

const find = (id: string) => instances.value.find(instance => instance.id === id) ?? null

const exportTarget = ref<Instance | null>(null)
const exportOpen = computed({
  get: () => !!exportTarget.value,
  set: (value: boolean) => {
    if (!value) exportTarget.value = null
  },
})

const removeTarget = ref<Instance | null>(null)
const removing = ref(false)

const { remove } = useInstanceActions()

async function confirmRemove() {
  const target = removeTarget.value
  if (!target || removing.value) return

  removing.value = true
  const result = await remove(target.id)
  removing.value = false

  if (!result.ok) return

  removeTarget.value = null

  toast.add({
    title: t('home.removed', { name: target.name }),
    color: 'success',
    icon: 'i-lucide-trash-2',
  })
}

function onImported(instanceId: string, updated: boolean) {
  importModalOpen.value = false
  announceImport(instanceId, updated)
}
</script>

<template>
  <div class="flex min-h-full w-full items-stretch">
    <div class="min-w-0 flex-1 px-8 pt-7 pb-8">
      <KitPageHeader
        :title="$t('home.title')"
        :meta="$t('home.count', instances.length)"
      >
        <KitSplitButton
          icon="i-lucide-plus"
          :menu-label="$t('home.more_ways')"
          @click="createModalOpen = true"
        >
          {{ $t('home.new_instance') }}

          <template #menu>
            <DropdownMenuItem
              icon="i-lucide-file-archive"
              @select="importModalOpen = true"
            >
              {{ $t('home.from_file') }}
            </DropdownMenuItem>
            <DropdownMenuItem
              icon="i-lucide-import"
              @select="launcherImportOpen = true"
            >
              {{ $t('home.from_launcher') }}
            </DropdownMenuItem>
          </template>
        </KitSplitButton>
      </KitPageHeader>

      <div
        v-if="instances.length"
        class="mt-4.5 flex items-center gap-2"
      >
        <KitSearchInput
          v-model="query"
          :placeholder="$t('home.search')"
          :aria-label="$t('home.search')"
          class="w-65"
        />

        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              variant="ghost"
              icon="i-lucide-arrow-down-wide-narrow"
              trailing-icon="i-lucide-chevron-down"
            >
              {{ sortLabel }}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuRadioGroup v-model="sort">
              <DropdownMenuRadioItem
                v-for="option in SORTS"
                :key="option"
                :value="option"
              >
                {{ $t(`home.sort.${option}`) }}
              </DropdownMenuRadioItem>
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>

        <span class="flex-1" />

        <RadioGroup
          v-model="view"
          :aria-label="$t('home.view.label')"
          class="w-fit"
        >
          <RadioGroupItem
            value="normal"
            :aria-label="$t('home.view.normal')"
            :title="$t('home.view.normal')"
            class="flex-none px-2"
          >
            <Icon
              name="i-lucide-layout-grid"
              class="size-4"
            />
          </RadioGroupItem>
          <RadioGroupItem
            value="compact"
            :aria-label="$t('home.view.compact')"
            :title="$t('home.view.compact')"
            class="flex-none px-2"
          >
            <Icon
              name="i-lucide-grid-3x3"
              class="size-4"
            />
          </RadioGroupItem>
        </RadioGroup>
      </div>

      <template v-if="shown.length">
        <div
          v-if="compact"
          class="mt-5.5 grid grid-cols-[repeat(auto-fill,minmax(6.5rem,1fr))] gap-1"
        >
          <InstanceCompactCard
            v-for="instance in shown"
            :key="instance.id"
            :instance="instance"
            :selected="instance.id === selectedId"
            @select="selectedId = $event"
            @export="exportTarget = find($event)"
            @remove="removeTarget = find($event)"
          />
        </div>

        <div
          v-else
          class="mt-6.5 grid grid-cols-[repeat(auto-fill,minmax(20rem,1fr))] gap-3"
        >
          <InstanceCard
            v-for="instance in shown"
            :key="instance.id"
            :instance="instance"
            @export="exportTarget = find($event)"
            @remove="removeTarget = find($event)"
          />
        </div>
      </template>

      <KitEmpty
        v-else-if="instances.length"
        icon="i-lucide-search-x"
        class="mt-6.5"
      >
        {{ $t('home.nothing_found') }}
      </KitEmpty>

      <KitEmpty
        v-else
        icon="i-lucide-box"
        class="mt-6.5"
      >
        {{ $t('home.empty') }}

        <template #action>
          <Button
            variant="primary"
            icon="i-lucide-plus"
            @click="createModalOpen = true"
          >
            {{ $t('home.new_instance') }}
          </Button>
          <Button
            icon="i-lucide-file-archive"
            @click="importModalOpen = true"
          >
            {{ $t('home.from_file') }}
          </Button>
        </template>
      </KitEmpty>
    </div>

    <InstanceSidePanel
      v-if="compact && instances.length"
      :instance="selected"
      class="sticky top-0 h-[calc(100vh-2.5rem)] w-75 shrink-0"
      @export="exportTarget = find($event)"
      @remove="removeTarget = find($event)"
    />

    <Dialog v-model:open="createModalOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ $t('home.create_title') }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <InstanceCreateModalBody @created="createModalOpen = false" />
        </DialogBody>
      </DialogContent>
    </Dialog>

    <Dialog v-model:open="importModalOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ $t('home.import_title') }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <ImportPackModalBody @imported="onImported" />
        </DialogBody>
      </DialogContent>
    </Dialog>

    <Dialog v-model:open="launcherImportOpen">
      <DialogContent class="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{{ $t('settings.import.title') }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <ImportWizard />
        </DialogBody>
      </DialogContent>
    </Dialog>

    <CastExportModal
      v-if="exportTarget"
      v-model:open="exportOpen"
      :instance="exportTarget"
    />

    <KitConfirmDialog
      :open="!!removeTarget"
      :title="$t('home.remove_title')"
      :description="$t('home.remove_text', { name: removeTarget?.name })"
      :confirm-label="$t('common.delete')"
      :loading="removing"
      @update:open="value => { if (!value) removeTarget = null }"
      @confirm="confirmRemove"
    />
  </div>
</template>
