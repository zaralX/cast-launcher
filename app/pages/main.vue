<script setup lang="ts">
import type { Instance } from '~/types/instance'

definePageMeta({
  layout: 'main',
})

const { t } = useI18n()
const instanceStore = useInstanceStore()
const toast = useToast()

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
    :class="compact ? 'flex items-stretch' : 'px-6 pb-10 pt-6 xl:px-10'"
  >
    <div
      class="min-w-0"
      :class="compact ? 'flex-1 px-6 pb-10 pt-6 xl:px-10' : ''"
    >
      <section>
        <AppSectionHeading
          index="01"
          :title="$t('home.title')"
        >
          <template #action>
            <AppButton
              class="group/imp ml-2 h-7 px-3 text-[10px]"
              tone="quiet"
              @click="importModalOpen = true"
            >
              <template #leading>
                <UIcon
                  name="i-lucide-file-archive"
                  class="size-3 transition-transform duration-500 group-hover/imp:-translate-y-0.5"
                />
              </template>
              {{ $t('home.from_file') }}
            </AppButton>

            <AppButton
              class="group/new ml-2 h-7 px-3 text-[10px]"
              @click="openCreateModal"
            >
              <template #leading>
                <UIcon
                  name="i-lucide-plus"
                  class="size-3 transition-transform duration-500 group-hover/new:rotate-90"
                />
              </template>
              {{ $t('home.create') }}
            </AppButton>
          </template>
        </AppSectionHeading>

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

          <button
            type="button"
            class="group flex cursor-pointer flex-col items-center justify-center gap-2 border border-dashed border-line p-2.5 text-fg-faint transition-colors duration-300 hover:border-acid/50 hover:text-acid"
            @click="openCreateModal"
          >
            <UIcon
              name="i-lucide-plus"
              class="size-4 transition-transform duration-500 ease-deck group-hover:rotate-90"
            />
            <span class="text-[11px] leading-tight">{{ $t('home.new') }}</span>
          </button>
        </div>

        <div
          v-else
          class="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5"
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

          <UButton
            color="neutral"
            variant="ghost"
            class="group min-h-[7rem] justify-center border border-dashed border-line text-fg-faint transition-all duration-500 ease-deck hover:-translate-y-0.5 hover:border-acid/50 hover:bg-transparent hover:text-acid"
            @click="openCreateModal"
          >
            <span class="flex flex-col items-center gap-2">
              <UIcon
                name="i-lucide-plus"
                class="size-4 transition-transform duration-500 ease-deck group-hover:rotate-90"
              />
              <span class="text-[10px] tracking-[0.2em]">{{ $t('home.new_instance') }}</span>
            </span>
          </UButton>
        </div>
      </section>
    </div>

    <InstanceSidePanel
      v-if="compact"
      :instance="selected"
      class="sticky top-0 h-[calc(100vh-2.75rem)] w-64 shrink-0"
      @remove="askRemove"
    />

    <UModal
      v-model:open="createModalOpen"
      :title="$t('home.create_title')"
    >
      <template #body>
        <InstanceCreateModalBody @created="createModalOpen = false" />
      </template>
    </UModal>

    <UModal
      v-model:open="importModalOpen"
      :title="$t('home.import_title')"
    >
      <template #body>
        <ImportPackModalBody @imported="onImported" />
      </template>
    </UModal>

    <UModal
      :open="!!removeTarget"
      :title="$t('home.remove_title')"
      @update:open="value => { if (!value) removeTarget = null }"
    >
      <template #body>
        <p class="text-[12px] leading-relaxed text-fg-muted">
          {{ $t('home.remove_text', { name: removeTarget?.name }) }}
        </p>
      </template>

      <template #footer>
        <div class="flex w-full items-center justify-end gap-3">
          <AppButton
            tone="quiet"
            class="text-[10px] tracking-[0.18em]"
            @click="removeTarget = null"
          >
            {{ $t('common.cancel') }}
          </AppButton>

          <AppButton
            class="h-8 text-[10px] tracking-[0.18em] hover:border-red-500 hover:before:bg-red-500 hover:text-white"
            icon="i-lucide-trash-2"
            :loading="removing"
            @click="confirmRemove"
          >
            {{ $t('common.delete') }}
          </AppButton>
        </div>
      </template>
    </UModal>
  </div>
</template>
