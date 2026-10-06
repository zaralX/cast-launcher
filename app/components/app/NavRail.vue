<script setup lang="ts">
const LINKS = [
  { labelKey: 'layout.nav.library', icon: 'i-lucide-box', to: '/main' },
  { labelKey: 'layout.nav.search', icon: 'i-lucide-search', to: '/search' },
  { labelKey: 'layout.nav.skins', icon: 'i-lucide-shirt', to: '/skins' },
  { labelKey: 'layout.nav.settings', icon: 'i-lucide-sliders-horizontal', to: '/settings' },
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
  <nav class="relative z-30 flex w-14 shrink-0 flex-col justify-between border-r border-line bg-ink-800/60">
    <div class="flex flex-col">
      <AppNavItem
        v-for="link in LINKS"
        :key="link.to"
        :to="link.to"
        :icon="link.icon"
        :label="$t(link.labelKey)"
      />
    </div>

    <div class="mb-2 flex flex-col items-center gap-2">
      <AppNavItem
        to="/credits"
        icon="i-lucide-info"
        :label="$t('layout.nav.credits')"
      />

      <Tooltip v-if="currentAccount">
        <TooltipTrigger as-child>
          <NuxtLink
            to="/settings"
            :aria-label="currentAccount.name"
            class="group grid size-10 place-items-center border border-line bg-ink-700 outline-none transition-colors duration-300 hover:border-acid/50 focus-visible:border-acid/50"
          >
            <img
              :src="`https://assets.zaralx.ru/api/v2/minecraft/players/${currentAccount.name}/face`"
              class="size-6 transition-transform duration-300 group-hover:scale-110"
              :alt="currentAccount.name"
              @error="fallbackFace"
            >
          </NuxtLink>
        </TooltipTrigger>
        <TooltipContent side="right">
          {{ currentAccount.name }}
        </TooltipContent>
      </Tooltip>
    </div>
  </nav>
</template>
