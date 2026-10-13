<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()
</script>

<template>
  <div class="flex items-stretch">
    <slot />

    <Button
      variant="ghost"
      size="bar"
      icon="i-lucide-minus"
      :aria-label="$t('layout.window.minimize')"
      @click="appWindow?.minimize()"
    />

    <Button
      variant="ghost"
      size="bar"
      :aria-label="$t('layout.window.maximize')"
      @click="appWindow?.toggleMaximize()"
    >
      <Icon
        name="i-lucide-square"
        class="size-3.25"
      />
    </Button>

    <Button
      variant="close"
      size="bar"
      icon="i-lucide-x"
      :aria-label="$t('layout.window.close')"
      @click="appWindow?.close()"
    />
  </div>
</template>
