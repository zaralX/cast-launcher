<script setup lang="ts">
import type { Instance } from '~/types/instance'

definePageMeta({
  layout: 'main',
})

const { t } = useI18n()
const instanceStore = useInstanceStore()
const toast = useAppToast()

const { installInstance, playInstance } = instanceStore
const { instances, installs } = storeToRefs(instanceStore)

const createModalOpen = ref(false)
const importModalOpen = ref(false)
const announceImport = useImportToast()

const compact = useCompact()

const selectedId = ref<string | null>(null)
const selected = computed(() => instances.value.find(instance => instance.id === selectedId.value) ?? null)

watch([compact, instances], () => {
  if (compact.value && !selected.value) selectedId.value = instances.value[0]?.id ?? null
}, { immediate: true })

const removeTarget = ref<Instance | null>(null)
const removing = ref(false)

const { remove } = useInstanceActions()

function askRemove(id: string) {
  removeTarget.value = instances.value.find(instance => instance.id === id) ?? null
}

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

const isRunning = (id: string) => instanceStore.isRunning(id)

const isInstalling = (id: string) => installs.value.some(install => install.instanceId === id)

const installOf = (id: string) => installs.value.find(i => i.instanceId === id)

const openCreateModal = () => {
  createModalOpen.value = true
}

function onImported(instanceId: string, updated: boolean) {
  importModalOpen.value = false
  announceImport(instanceId, updated)
}

const run = (id: string) => safeRun(
  () => playInstance(id),
  { context: { instanceId: id, action: t('instance.context.play') } },
)
</script>

<template>
  <div
    :data-compact="compact"
    class="min-h-full w-full"
    :class="compact ? 'flex items-stretch' : 'px-6 pt-6 pb-10 xl:px-10'"
  >
    <div
      class="min-w-0"
      :class="compact ? 'flex-1 px-6 pt-6 pb-10 xl:px-10' : ''"
    >
      <section>
        <KitSectionHeading
          index="01"
          :title="$t('home.title')"
        >
          <template #action>
            <Button
              variant="quiet"
              size="xs"
              class="group/imp ml-2"
              @click="importModalOpen = true"
            >
              <Icon
                name="i-lucide-file-archive"
                class="size-3 transition-transform duration-500 group-hover/imp:-translate-y-0.5"
              />
              {{ $t('home.from_file') }}
            </Button>

            <Button
              size="xs"
              class="group/new ml-2"
              @click="openCreateModal"
            >
              <Icon
                name="i-lucide-plus"
                class="size-3 transition-transform duration-500 group-hover/new:rotate-90"
              />
              {{ $t('home.create') }}
            </Button>
          </template>
        </KitSectionHeading>

        <div
          v-if="compact"
          class="mt-4 grid grid-cols-[repeat(auto-fill,minmax(6.25rem,1fr))] gap-1"
        >
          <InstanceCompactCard
            v-for="instance in instances"
            :key="instance.id"
            :instance="instance"
            :selected="instance.id === selectedId"
            @select="selectedId = $event"
            @remove="askRemove"
          />

          <Button
            variant="dashed"
            class="group h-auto flex-col gap-2 p-2.5 normal-case tracking-normal"
            @click="openCreateModal"
          >
            <Icon
              name="i-lucide-plus"
              class="size-4 transition-transform duration-500 ease-deck group-hover:rotate-90"
            />
            <span class="font-sans text-caption leading-tight">{{ $t('home.new') }}</span>
          </Button>
        </div>

        <div
          v-else
          class="mt-4 grid grid-cols-3 gap-3 xl:grid-cols-4 2xl:grid-cols-5"
        >
          <InstanceCard
            v-for="(instance, i) in instances"
            :key="instance.id"
            :instance="instance"
            :running="isRunning(instance.id)"
            :installing="isInstalling(instance.id)"
            :progress="installOf(instance.id)?.progress"
            :phase="installOf(instance.id)?.phase"
            class="animate-rise"
            :style="{ animationDelay: `${i * 35}ms` }"
            @install="installInstance"
            @run="run"
          />

          <Button
            variant="dashed"
            class="group h-auto min-h-28 flex-col gap-2 duration-500 ease-deck hover:-translate-y-0.5"
            @click="openCreateModal"
          >
            <Icon
              name="i-lucide-plus"
              class="size-4 transition-transform duration-500 ease-deck group-hover:rotate-90"
            />
            {{ $t('home.new_instance') }}
          </Button>
        </div>
      </section>
    </div>

    <InstanceSidePanel
      v-if="compact"
      :instance="selected"
      class="sticky top-0 h-[calc(100vh-2.75rem)] w-64 shrink-0"
      @remove="askRemove"
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
