<script setup lang="ts">
import CreateInstanceModalBody from "~/components/CreateInstanceModalBody.vue";
import {useCastPackStore} from "~/stores/castpack";

definePageMeta({
  layout: "main"
})

const instanceStore = useInstanceStore()
const castpackStore = useCastPackStore()
const toast = useToast()

const {installInstance, playInstance} = instanceStore
const {running, instances, installs} = storeToRefs(instanceStore)

const createModalOpen = ref(false)

const compact = useCompact()

const catalogPacks = computed(() =>
    castpackStore.packs.filter(pack => !castpackStore.instanceOf(pack.id))
)

const isRunning = (id: string) => running.value.some(game => game.instanceId === id)

const isInstalling = (id: string) => installs.value.some(install => install.instanceId === id)

const installOf = (id: string) => installs.value.find(i => i.instanceId === id)

const openCreateModal = () => {
  createModalOpen.value = true
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
  <div :data-compact="compact" class="min-h-full w-full px-6 pb-10 pt-6 xl:px-10">
    <section>
      <SectionHeading index="01" title="Ваши сборки">
        <template #action>
          <AppButton class="group/new ml-2 h-7 px-3 text-[10px]" @click="openCreateModal">
            <template #leading>
              <UIcon name="i-lucide-plus" class="size-3 transition-transform duration-500 group-hover/new:rotate-90"/>
            </template>
            Создать
          </AppButton>
        </template>
      </SectionHeading>

      <div class="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
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

      <div class="mt-4 grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
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

    <UModal v-model:open="createModalOpen" title="Создание сборки">
      <template #body>
        <CreateInstanceModalBody @created="createModalOpen = false"/>
      </template>
    </UModal>
  </div>
</template>
