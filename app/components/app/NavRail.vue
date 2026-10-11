<script setup lang="ts">
const LINKS = [
  { labelKey: 'layout.nav.library', icon: 'i-lucide-box', to: '/main', match: ['/instance'] },
  { labelKey: 'layout.nav.search', icon: 'i-lucide-compass', to: '/search' },
  { labelKey: 'layout.nav.skins', icon: 'i-lucide-shirt', to: '/skins' },
]
</script>

<template>
  <nav
    :aria-label="$t('layout.nav.label')"
    class="flex w-14 shrink-0 flex-col items-center gap-1 pt-1.5 pb-3.5"
  >
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
      to="/settings"
      icon="i-lucide-settings"
      :label="$t('layout.nav.settings')"
      :match="['/credits']"
    />

    <AppAccountMenu />
  </nav>
</template>
