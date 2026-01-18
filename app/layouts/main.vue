<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'

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
</script>

<template>
<div class="h-screen w-full">
  <div class="flex w-full">
    <div class="bg-zinc-800/50 w-16">
      <div class="flex flex-col w-16 h-16 items-center justify-center gap-1">
        <Icon name="lucide:house" class="text-lg" />
        <p class="text-sm">Главная</p>
      </div>
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
    <UButton size="xl" class="ml-auto mr-0 font-unbounded" icon="i-lucide-play">Играть</UButton>
  </div>
</div>
</template>

<style scoped>

</style>