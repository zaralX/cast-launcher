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
</script>

<template>
  <div class="min-h-full w-full px-8 pb-16 pt-10 xl:px-14">
    <header class="grid gap-8 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
      <div class="animate-rise">
        <h1 class="mt-4 font-unbounded text-[clamp(30px,4.2vw,46px)] font-bold leading-[0.92] tracking-[-0.055em] text-fg">
          Привет здоровяк<span class="text-acid">!</span>
        </h1>
      </div>
    </header>

    <section :class="myPacks.length ? 'mt-16' : 'mt-14'">
      <SectionHeading
          index="01"
          title="Ваши сборки"
          :meta="`${installedCount} / ${instances.length} установлено`"
      >
        <template #action>
          <button
              type="button"
              class="group/new relative ml-2 flex h-8 items-center gap-2 overflow-hidden border border-line px-3.5 font-mono text-[10px] uppercase tracking-[0.18em] text-fg transition-colors duration-300 hover:border-acid hover:text-on-acid"
              @click="createModalOpen = true"
          >
            <span
                class="absolute inset-0 origin-left scale-x-0 bg-acid transition-transform duration-500 ease-deck group-hover/new:scale-x-100"
                aria-hidden="true"
            />
            <span class="relative flex items-center gap-2">
              <UIcon name="i-lucide-plus" class="size-3.5 transition-transform duration-500 group-hover/new:rotate-90"/>
              Создать
            </span>
          </button>
        </template>
      </SectionHeading>

      <div class="mt-7 grid gap-5 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
        <InstanceCard
            v-for="(instance, i) in instances"
            :key="instance.id"
            :instance="instance"
            :running="isRunning(instance.id)"
            :installing="isInstalling(instance.id)"
            :progress="installOf(instance.id)?.progress"
            :phase="installOf(instance.id)?.phase"
            class="animate-rise"
            :style="{ animationDelay: `${i * 45}ms` }"
            @install="installInstance"
            @run="runInstance"
        />

        <button
            type="button"
            class="group grid min-h-[13.5rem] place-items-center border border-dashed border-line text-fg-faint transition-all duration-500 ease-deck hover:-translate-y-1 hover:border-acid/50 hover:text-acid"
            @click="createModalOpen = true"
        >
          <span class="flex flex-col items-center gap-3">
            <UIcon
                name="i-lucide-plus"
                class="size-5 transition-transform duration-500 ease-deck group-hover:rotate-90"
            />
            <span class="font-mono text-[10px] uppercase tracking-[0.2em]">Новая сборка</span>
          </span>
        </button>
      </div>
    </section>

    <section v-if="myPacks.length" class="mt-14">
      <SectionHeading index="02" title="Сборки от zaralX" :meta="`${myPacks.length} шт.`"/>

      <div class="mt-7 grid gap-5 lg:grid-cols-2 2xl:grid-cols-3">
        <PackCard
            v-for="([packId, pack], i) in myPacks"
            :key="packId"
            :pack="pack"
            :pack-id="packId"
            class="animate-rise"
            :style="{ animationDelay: `${i * 60}ms` }"
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
