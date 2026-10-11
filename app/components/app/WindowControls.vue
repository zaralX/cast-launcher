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
      class="group h-10 w-11.5 rounded-none"
      @click="appWindow?.minimize()"
    >
      <span class="h-px w-3 bg-current" />
    </Button>

    <Button
      variant="ghost"
      size="icon-lg"
      :aria-label="$t('layout.window.maximize')"
      class="group h-10 w-11.5 rounded-none"
      @click="appWindow?.toggleMaximize()"
    >
      <span class="size-2.5 rounded-[2px] border border-current" />
    </Button>

    <Button
      variant="ghost"
      size="icon-lg"
      :aria-label="$t('layout.window.close')"
      class="group h-10 w-11.5 rounded-none hover:bg-danger hover:text-on-danger"
      @click="appWindow?.close()"
    >
      <Icon
        name="i-lucide-x"
        class="size-4"
      />
    </Button>
  </div>
</template>
