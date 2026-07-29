<script setup lang="ts">
import type {CatalogPack} from "~/types/castpack"
import {useCastPackStore} from "~/stores/castpack"

definePageMeta({
  layout: "main"
})

const store = useCastPackStore()
const instanceStore = useInstanceStore()
const toast = useToast()

const {catalog, loading, loaded} = storeToRefs(store)

const query = ref("")

const packs = computed<CatalogPack[]>(() => {
  const needle = query.value.trim().toLowerCase()
  if (!needle) return store.packs

  return store.packs.filter(pack =>
      [pack.name, pack.summary, pack.description, pack.id, ...pack.tags]
          .join(" ")
          .toLowerCase()
          .includes(needle)
  )
})

const outdated = computed(() => store.packs.filter(pack => store.stateOf(pack) === "outdated").length)

const installOf = (packId: string) => {
  const instance = store.instanceOf(packId)
  return instance ? instanceStore.getInstall(instance.id) : undefined
}

async function load(force = false) {
  await safeRun(() => store.loadCatalog(force), {
    code: "NETWORK",
    context: {action: "Загрузка каталога CastPack"}
  })
}

onMounted(() => load())

async function install(packId: string) {
  const pack = store.packs.find(item => item.id === packId)

  const started = await attempt(() => store.installPack(packId), {
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

const play = (instanceId: string) => safeRun(
    () => instanceStore.playInstance(instanceId),
    {context: {instanceId, action: "Запуск сборки"}}
)
</script>

<template>
  <div class="min-h-full w-full px-6 pb-10 pt-6 xl:px-10">
    <header class="animate-rise flex flex-wrap items-end justify-between gap-4">
      <div>
        <p class="font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">Каталог</p>
        <h1 class="mt-3 font-unbounded text-lg font-bold leading-none tracking-[-0.055em] text-fg xl:text-xl">
          CastPacks<span class="text-acid">.</span>
        </h1>
      </div>

      <div class="flex items-center gap-3">
        <UInput
            v-model="query"
            placeholder="Поиск по каталогу"
            icon="i-lucide-search"
            class="w-56"
        />

        <AppButton
            tone="quiet"
            class="h-9 shrink-0 text-[10px] tracking-[0.18em]"
            icon="i-lucide-rotate-cw"
            :loading="loading"
            @click="load(true)"
        >
          Обновить
        </AppButton>
      </div>
    </header>

    <p
        v-if="outdated"
        class="animate-rise mt-6 border border-amber-400/30 bg-amber-400/[0.04] px-4 py-3 text-[12px] leading-relaxed text-fg-muted"
    >
      Обновления есть у {{ outdated }} сборок — они докачаются сами при нажатии «Играть».
    </p>

    <div v-if="loading && !loaded" class="flex flex-col items-center gap-4 py-20">
      <span class="relative block h-px w-40 overflow-hidden bg-line">
        <span class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"/>
      </span>
      <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Загрузка каталога</p>
    </div>

    <div v-else-if="!packs.length" class="flex items-center gap-3 py-20">
      <span class="size-1.5 bg-fg-faint animate-blink"/>
      <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
        {{ query ? 'Ничего не нашлось' : 'Каталог пуст' }}
      </p>
    </div>

    <div v-else class="mt-6 grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
      <CastpackCard
          v-for="(pack, i) in packs"
          :key="pack.id"
          :pack="pack"
          :state="store.stateOf(pack)"
          :icon="store.instanceOf(pack.id)?.icon"
          :instance-id="store.instanceOf(pack.id)?.id"
          :progress="installOf(pack.id)?.progress"
          :phase="installOf(pack.id)?.phase"
          class="animate-rise"
          :style="{ animationDelay: `${i * 45}ms` }"
          @install="install"
          @play="play"
      />
    </div>

    <p v-if="catalog?.updatedAt" class="mt-8 font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
      Каталог обновлён: {{ catalog.updatedAt }}
    </p>
  </div>
</template>
