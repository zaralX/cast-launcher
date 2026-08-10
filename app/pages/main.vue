<script setup lang="ts">
import CreateInstanceModalBody from "~/components/CreateInstanceModalBody.vue";
import ImportPackModalBody from "~/components/ImportPackModalBody.vue";
import {useCastPackStore} from "~/stores/castpack";
import type {Instance} from "~/types/instance";

definePageMeta({
  layout: "main"
})

const instanceStore = useInstanceStore()
const castpackStore = useCastPackStore()
const toast = useToast()

const {installInstance, playInstance} = instanceStore
const {instances, installs} = storeToRefs(instanceStore)

const createModalOpen = ref(false)
const importModalOpen = ref(false)

const compact = useCompact()

const selectedId = ref<string | null>(null)
const selected = computed(() => instances.value.find(instance => instance.id === selectedId.value) ?? null)

watch([compact, instances], () => {
  if (compact.value && !selected.value) selectedId.value = instances.value[0]?.id ?? null
}, {immediate: true})

const removeTarget = ref<Instance | null>(null)
const removing = ref(false)

const {remove} = useInstanceActions()

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
    title: `Сборка «${target.name}» удалена`,
    color: "success",
    icon: "i-lucide-trash-2"
  })
}

const catalogPacks = computed(() =>
    castpackStore.packs.filter(pack => !castpackStore.instanceOf(pack.id))
)

const isRunning = (id: string) => instanceStore.isRunning(id)

const isInstalling = (id: string) => installs.value.some(install => install.instanceId === id)

const installOf = (id: string) => installs.value.find(i => i.instanceId === id)

const openCreateModal = () => {
  createModalOpen.value = true
}

function onImported(instanceId: string) {
  importModalOpen.value = false

  const instance = instances.value.find(item => item.id === instanceId)

  toast.add({
    title: `Сборка «${instance?.name ?? "из файла"}» импортирована`,
    description: "Файлы модпака скачиваются в фоне",
    color: "success",
    icon: "i-lucide-file-archive"
  })
}

onMounted(() => safeRun(() => castpackStore.loadCatalog(), {
  code: "NETWORK",
  context: {action: "Загрузка каталога CastPack"}
}))

const run = (id: string) => safeRun(
    () => playInstance(id),
    {context: {instanceId: id, action: "Запуск сборки"}}
)

async function installPack(packId: string) {
  const pack = castpackStore.packs.find(item => item.id === packId)

  const started = await attempt(() => castpackStore.installPack(packId), {
    context: {action: "Установка сборки CastPack", packId}
  })

  if (!started.ok) return

  toast.add({
    title: `Установка «${pack?.name ?? packId}»`,
    description: "Файлы сборки скачиваются в фоне",
    color: "success",
    icon: "i-lucide-arrow-down-to-line"
  })
}
</script>

<template>
  <div
      :data-compact="compact"
      class="min-h-full w-full"
      :class="compact ? 'flex items-stretch' : 'px-6 pb-10 pt-6 xl:px-10'"
  >
    <div class="min-w-0" :class="compact ? 'flex-1 px-6 pb-10 pt-6 xl:px-10' : ''">
      <section>
        <SectionHeading index="01" title="Ваши сборки">
          <template #action>
            <AppButton class="group/imp ml-2 h-7 px-3 text-[10px]" tone="quiet" @click="importModalOpen = true">
              <template #leading>
                <UIcon
                    name="i-lucide-file-archive"
                    class="size-3 transition-transform duration-500 group-hover/imp:-translate-y-0.5"
                />
              </template>
              Из файла
            </AppButton>

            <AppButton class="group/new ml-2 h-7 px-3 text-[10px]" @click="openCreateModal">
              <template #leading>
                <UIcon name="i-lucide-plus" class="size-3 transition-transform duration-500 group-hover/new:rotate-90"/>
              </template>
              Создать
            </AppButton>
          </template>
        </SectionHeading>

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
            <span class="text-[11px] leading-tight">Новая</span>
          </button>
        </div>

        <div v-else class="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
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
              <span class="text-[10px] tracking-[0.2em]">Новая сборка</span>
            </span>
          </UButton>
        </div>
      </section>

      <section v-if="catalogPacks.length" class="mt-8">
        <SectionHeading index="02" title="Сборки CastPack" :meta="`${catalogPacks.length} шт.`">
          <template #action>
            <NuxtLink
                to="/castpack"
                class="group/all ml-2 inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint transition-colors duration-300 hover:text-acid"
            >
              Весь каталог
              <UIcon
                  name="i-lucide-arrow-right"
                  class="size-3 transition-transform duration-500 ease-deck group-hover/all:translate-x-1"
              />
            </NuxtLink>
          </template>
        </SectionHeading>

        <div class="mt-4 grid gap-3" :class="compact ? '2xl:grid-cols-2' : 'lg:grid-cols-2 2xl:grid-cols-3'">
          <CastpackCard
              v-for="(pack, i) in catalogPacks"
              :key="pack.id"
              :pack="pack"
              :state="castpackStore.stateOf(pack)"
              class="animate-rise"
              :style="{ animationDelay: `${i * 45}ms` }"
              @install="installPack"
          />
        </div>
      </section>
    </div>

    <InstanceSidePanel
        v-if="compact"
        :instance="selected"
        class="sticky top-0 h-[calc(100vh-2.75rem)] w-64 shrink-0"
        @remove="askRemove"
    />

    <UModal v-model:open="createModalOpen" title="Создание сборки">
      <template #body>
        <CreateInstanceModalBody @created="createModalOpen = false"/>
      </template>
    </UModal>

    <UModal v-model:open="importModalOpen" title="Импорт модпака из файла">
      <template #body>
        <ImportPackModalBody @imported="onImported"/>
      </template>
    </UModal>

    <UModal
        :open="!!removeTarget"
        title="Удаление сборки"
        @update:open="value => { if (!value) removeTarget = null }"
    >
      <template #body>
        <p class="text-[12px] leading-relaxed text-fg-muted">
          Сборка «{{ removeTarget?.name }}» и все её файлы будут удалены безвозвратно.
        </p>
      </template>

      <template #footer>
        <div class="flex w-full items-center justify-end gap-3">
          <AppButton tone="quiet" class="text-[10px] tracking-[0.18em]" @click="removeTarget = null">
            Отмена
          </AppButton>

          <AppButton
              class="h-8 text-[10px] tracking-[0.18em] hover:border-red-500 hover:before:bg-red-500 hover:text-white"
              icon="i-lucide-trash-2"
              :loading="removing"
              @click="confirmRemove"
          >
            Удалить
          </AppButton>
        </div>
      </template>
    </UModal>
  </div>
</template>
