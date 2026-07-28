<script setup lang="ts">
import CreateInstanceModalBody from "~/components/CreateInstanceModalBody.vue";
import {useAppStore} from "~/stores/app";

definePageMeta({
  layout: "main"
})

const appStore = useAppStore()
const instanceStore = useInstanceStore()
const {installInstance, runInstance} = instanceStore
const {running, instances, installs} = storeToRefs(instanceStore)

const createModalOpen = ref(false)

const myPacks = computed(() => Object.entries(appStore.myPacksConfig?.packs ?? {}))

const installedCount = computed(() => instances.value.filter(i => i.installed).length)

const isRunning = (id: string) => running.value.some(game => game.instanceId === id)

const isInstalling = (id: string) => installs.value.some(install => install.instanceId === id)

const installOf = (id: string) => installs.value.find(i => i.instanceId === id)

const openCreateModal = () => {
  createModalOpen.value = true
}
</script>

<template>
  <div class="min-h-full w-full px-6 pb-10 pt-6 xl:px-10">
    <header class="animate-rise flex flex-wrap items-baseline gap-x-4 gap-y-1">
      <h1 class="font-unbounded text-lg font-bold leading-none tracking-[-0.055em] text-fg xl:text-xl">
        Привет здоровяк<span class="text-acid">!</span>
      </h1>
    </header>

    <section class="mt-6">
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
            @run="runInstance"
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

    <section v-if="myPacks.length" class="mt-8">
      <SectionHeading index="02" title="Сборки от zaralX" :meta="`${myPacks.length} шт.`"/>

      <div class="mt-4 grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
        <PackCard
            v-for="([packId, pack], i) in myPacks"
            :key="packId"
            :pack="pack"
            :pack-id="packId"
            class="animate-rise"
            :style="{ animationDelay: `${i * 45}ms` }"
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
