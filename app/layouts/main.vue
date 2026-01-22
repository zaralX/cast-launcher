<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { getCurrentWindow } from "@tauri-apps/api/window";
import ActiveDownloadingModal from "~/components/ActiveDownloadingModal.vue";

const items = ref<DropdownMenuItem[]>([
  {
    label: 'Account',
    icon: 'i-lucide-user'
  },
  {
    label: 'Account',
    icon: 'i-lucide-user'
  },
  {
    label: 'Account',
    icon: 'i-lucide-user'
  },
])

const links = [{
  name: "Главная",
  icon: "lucide:house",
  to: "/main"
},{
  name: "Настройки",
  icon: "lucide:cog",
  to: "/settings"
}]

const route = useRoute()

const appWindow = getCurrentWindow();
</script>

<template>
<div class="h-screen w-full">
  <div class="bg-zinc-950/50 h-12 flex">
    <div class="flex">
      <NuxtImg src="/logo.png" class="h-12 w-12" />
      <ActiveDownloadingModal />
    </div>
    <div data-tauri-drag-region class="flex-1"></div>
    <div>
      <UButton icon="i-lucide-minimize-2" class="h-full aspect-square justify-center rounded-none" variant="ghost" color="neutral" @click="appWindow?.minimize()" />
      <UButton icon="i-lucide-scaling" class="h-full aspect-square justify-center rounded-none" variant="ghost" color="neutral" @click="appWindow?.toggleMaximize()" />
      <UButton icon="i-lucide-x" class="h-full aspect-square justify-center rounded-none" variant="ghost" color="neutral" @click="appWindow?.close()" />
    </div>
  </div>
  <div class="flex w-full">
    <div class="bg-zinc-800/50 w-12">
      <NuxtLink v-for="link in links" :to="link.to" :class="{'text-sky-500 bg-blue-500/25': link.to == route.path}" class="flex flex-col w-12 h-12 items-center justify-center gap-1">
        <Icon :name="link.icon" class="text-lg" />
<!--        <p class="text-sm">{{ link.name }}</p>-->
      </NuxtLink>
    </div>
    <UScrollArea class="h-[calc(100vh-3rem)] w-full">
      <slot />
    </UScrollArea>
  </div>
</div>
</template>

<style scoped>

</style>