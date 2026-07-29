<script setup lang="ts">
import {getCurrentWindow} from "@tauri-apps/api/window";
import {getVersion} from "@tauri-apps/api/app";
import ActiveDownloadingModal from "~/components/ActiveDownloadingModal.vue";
import BlockedFilesModal from "~/components/BlockedFilesModal.vue";

const instanceStore = useInstanceStore()

const awaiting = computed(() => instanceStore.installs.find(install => install.awaitingFiles) ?? null)

const links = [{
  name: "Библиотека",
  icon: "i-lucide-box",
  to: "/main"
}, {
  name: "Поиск сборок",
  icon: "i-lucide-search",
  to: "/search"
}, {
  name: "Система",
  icon: "i-lucide-sliders-horizontal",
  to: "/settings"
}]

const route = useRoute()
const appWindow = getCurrentWindow()

const accountStore = useAccountStore()
const {accountConfig} = storeToRefs(accountStore)

const currentAccount = computed(() => {
  const cfg = accountConfig.value
  if (!cfg?.accounts?.length) return null
  return cfg.accounts[cfg.selected ?? 0] ?? null
})

const version = ref("")
onMounted(async () => {
  try {
    version.value = await getVersion()
  } catch {
    version.value = ""
  }
})
</script>

<template>
  <div class="flex h-screen w-full flex-col overflow-hidden bg-ink-900 text-fg">
    <header class="relative z-40 flex h-11 shrink-0 items-stretch border-b border-line bg-ink-800">
      <div class="flex items-center gap-3 pl-3 pr-6">
        <NuxtImg src="/logo.png" class="h-6 w-6" alt=""/>
        <p class="font-unbounded text-[13px] font-semibold leading-none tracking-[-0.05em]">
          CAST<span class="text-acid">.</span>
        </p>
        <span v-if="version" class="font-mono text-[10px] leading-none text-fg-faint">v{{ version }}</span>
      </div>

      <div data-tauri-drag-region class="flex flex-1 items-center justify-center px-4">
        <ActiveDownloadingModal/>
        <BlockedFilesModal v-if="awaiting" :key="awaiting.instanceId" :install="awaiting"/>
      </div>

      <div class="flex items-stretch">
        <ErrorCenter/>

        <UButton
            color="neutral"
            variant="ghost"
            aria-label="Свернуть"
            class="group h-11 w-11 justify-center text-fg-faint hover:bg-ink-600 hover:text-fg"
            @click="appWindow?.minimize()"
        >
          <span class="h-px w-3.5 bg-current transition-transform duration-300 group-hover:scale-x-75"/>
        </UButton>

        <UButton
            color="neutral"
            variant="ghost"
            aria-label="Развернуть"
            class="group h-11 w-11 justify-center text-fg-faint hover:bg-ink-600 hover:text-fg"
            @click="appWindow?.toggleMaximize()"
        >
          <span class="size-2.5 border border-current transition-all duration-300 group-hover:size-3"/>
        </UButton>

        <UButton
            color="neutral"
            variant="ghost"
            aria-label="Закрыть"
            class="group h-11 w-11 justify-center text-fg-faint hover:bg-red-500 hover:text-white"
            @click="appWindow?.close()"
        >
          <UIcon name="i-lucide-x" class="size-3.5 transition-transform duration-300 group-hover:rotate-90"/>
        </UButton>
      </div>
    </header>

    <div class="flex min-h-0 flex-1">
      <nav class="relative z-30 flex w-16 shrink-0 flex-col justify-between border-r border-line bg-ink-800/60 py-5">
        <div class="flex flex-col">
          <NuxtLink
              v-for="(link, i) in links"
              :key="link.to"
              :to="link.to"
              class="group relative grid h-16 w-16 place-items-center"
          >
            <span
                class="absolute left-0 top-1/2 w-px -translate-y-1/2 bg-acid transition-all duration-500 ease-deck"
                :class="route.path === link.to ? 'h-8 opacity-100' : 'h-0 opacity-0 group-hover:h-4 group-hover:opacity-60'"
            />

            <span
                class="absolute right-2.5 top-3.5 font-mono text-[9px] leading-none transition-colors duration-300"
                :class="route.path === link.to ? 'text-acid' : 'text-fg-faint/50 group-hover:text-fg-faint'"
            >
              {{ String(i + 1).padStart(2, "0") }}
            </span>

            <UIcon
                :name="link.icon"
                class="size-[18px] transition-all duration-300 ease-deck"
                :class="route.path === link.to
                  ? 'text-acid'
                  : 'text-fg-faint group-hover:-translate-y-0.5 group-hover:text-fg'"
            />

            <span
                class="pointer-events-none absolute left-[calc(100%+10px)] whitespace-nowrap border border-line bg-ink-700 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-fg opacity-0 -translate-x-2 transition-all duration-300 ease-deck group-hover:translate-x-0 group-hover:opacity-100"
            >
              {{ link.name }}
            </span>
          </NuxtLink>
        </div>

        <NuxtLink
            v-if="currentAccount"
            to="/settings"
            class="group relative mx-auto grid size-10 place-items-center border border-line bg-ink-700 transition-colors duration-300 hover:border-acid/50"
        >
          <NuxtImg
              :src="`https://assets.zaralx.ru/api/v1/minecraft/vanilla/player/face/${currentAccount.name}/full`"
              class="size-6 transition-transform duration-300 group-hover:scale-110"
              :alt="currentAccount.name"
          />
          <span
              class="pointer-events-none absolute bottom-0 left-[calc(100%+10px)] whitespace-nowrap border border-line bg-ink-700 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-fg opacity-0 -translate-x-2 transition-all duration-300 ease-deck group-hover:translate-x-0 group-hover:opacity-100"
          >
            {{ currentAccount.name }}
          </span>
        </NuxtLink>
      </nav>

      <main class="relative min-w-0 flex-1">
        <div
            class="pointer-events-none absolute -left-32 -top-32 z-10 h-96 w-96 rounded-full bg-acid/[0.05] blur-[130px]"
            aria-hidden="true"
        />
        <UScrollArea class="h-[calc(100vh-2.75rem)] w-full">
          <slot/>
        </UScrollArea>
      </main>
    </div>
  </div>
</template>
