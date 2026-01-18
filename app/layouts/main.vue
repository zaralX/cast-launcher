<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
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
</script>

<template>
<div class="h-screen w-full">
  <div class="flex w-full">
    <div class="bg-zinc-800/50 w-24">
      <NuxtLink v-for="link in links" :to="link.to" :class="{'text-sky-500': link.to == route.path}" class="flex flex-col w-24 h-16 items-center justify-center gap-1">
        <Icon :name="link.icon" class="text-lg" />
        <p class="text-sm">{{ link.name }}</p>
      </NuxtLink>
    </div>
    <div class="h-[calc(100vh-4rem)] flex w-full">
      <slot />
    </div>
  </div>
  <div class="bg-zinc-950/50 h-16 p-2 flex gap-4 items-center">
    <UDropdownMenu :items="items">
      <UFieldGroup>
        <UButton color="neutral" variant="subtle" class="min-w-48">
          <NuxtImg src="/default_skin_face.png" class="w-8 h-8 rounded-lg" />
          <p>123123</p>
        </UButton>
        <UButton color="neutral" variant="outline" icon="i-lucide-chevron-up" />
      </UFieldGroup>
    </UDropdownMenu>
    <UDropdownMenu :items="items">
      <UFieldGroup>
        <UButton color="neutral" variant="subtle" class="min-w-48">
          <NuxtImg src="/default_skin_face.png" class="w-8 h-8 rounded-lg" />
          <p>123123</p>
        </UButton>
        <UButton color="neutral" variant="outline" icon="i-lucide-chevron-up" />
      </UFieldGroup>
    </UDropdownMenu>
    <ActiveDownloadingModal />
    <UButton size="xl" class="font-unbounded" icon="i-lucide-play">Играть</UButton>
  </div>
</div>
</template>

<style scoped>

</style>