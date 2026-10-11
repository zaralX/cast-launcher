<script setup lang="ts">
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

const LINKS = [
  { labelKey: 'layout.nav.library', icon: 'i-lucide-box', to: '/main', match: ['/instance'] },
  { labelKey: 'layout.nav.search', icon: 'i-lucide-compass', to: '/search' },
  { labelKey: 'layout.nav.skins', icon: 'i-lucide-shirt', to: '/skins' },
]

const accountStore = useAccountStore()
const { accountConfig } = storeToRefs(accountStore)

const currentAccount = computed(() => {
  const cfg = accountConfig.value
  if (!cfg?.accounts?.length) return null
  return cfg.accounts[cfg.selected ?? 0] ?? null
})
</script>

<template>
  <nav class="flex w-14 shrink-0 flex-col items-center gap-1 pt-1.5 pb-3.5">
    <AppNavItem
      v-for="link in LINKS"
      :key="link.to"
      :to="link.to"
      :icon="link.icon"
      :label="$t(link.labelKey)"
      :match="link.match"
    />

    <span class="flex-1" />

    <AppNavItem
      to="/credits"
      icon="i-lucide-info"
      :label="$t('layout.nav.credits')"
    />
    <AppNavItem
      to="/settings"
      icon="i-lucide-settings"
      :label="$t('layout.nav.settings')"
    />

    <Tooltip v-if="currentAccount">
      <TooltipTrigger as-child>
        <NuxtLink
          to="/settings"
          :aria-label="currentAccount.name"
          :class="cn(FOCUS_RING, 'mt-1.5 block size-8 overflow-hidden rounded-sm bg-control transition-opacity duration-150 hover:opacity-85')"
        >
          <img
            :src="`https://assets.zaralx.ru/api/v2/minecraft/players/${currentAccount.name}/face`"
            class="size-full [image-rendering:pixelated]"
            :alt="currentAccount.name"
            @error="fallbackFace"
          >
        </NuxtLink>
      </TooltipTrigger>
      <TooltipContent side="right">
        {{ currentAccount.name }}
      </TooltipContent>
    </Tooltip>
  </nav>
</template>
