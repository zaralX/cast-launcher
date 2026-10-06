<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()
</script>

<template>
  <div class="flex items-stretch">
    <slot />

    <Button
      variant="ghost"
      size="icon-lg"
      :aria-label="$t('layout.window.minimize')"
      class="group"
      @click="appWindow?.minimize()"
    >
      <span class="h-px w-3.5 bg-current transition-transform duration-300 group-hover:scale-x-75" />
    </Button>

    <Button
      variant="ghost"
      size="icon-lg"
      :aria-label="$t('layout.window.maximize')"
      class="group"
      @click="appWindow?.toggleMaximize()"
    >
      <span class="size-2.5 border border-current transition-all duration-300 group-hover:size-3" />
    </Button>

    <Button
      variant="ghost"
      size="icon-lg"
      :aria-label="$t('layout.window.close')"
      class="group hover:bg-danger-strong hover:text-on-danger"
      @click="appWindow?.close()"
    >
      <Icon
        name="i-lucide-x"
        class="size-3.5 transition-transform duration-300 group-hover:rotate-90"
      />
    </Button>
  </div>
</template>
