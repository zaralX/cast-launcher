<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getVersion } from '@tauri-apps/api/app'

const instanceStore = useInstanceStore()

const awaiting = computed(() => instanceStore.installs.find(install => install.awaitingFiles) ?? null)

const appWindow = getCurrentWindow()

const version = ref('')
onMounted(async () => {
  try {
    version.value = await getVersion()
  }
  catch {
    version.value = ''
  }
})
</script>

<template>
  <header class="relative z-40 flex h-11 shrink-0 items-stretch border-b border-line bg-ink-800">
    <div class="pointer-events-none flex items-center gap-3 pr-6 pl-3 select-none">
      <img
        src="/logo.svg"
        class="size-8"
        alt=""
      >
      <p class="font-unbounded text-title font-semibold leading-none tracking-[-0.05em]">
        CAST<span class="text-acid">.</span>
      </p>
      <span
        v-if="version"
        class="font-mono text-label leading-none text-fg-faint"
      >v{{ version }}</span>
    </div>

    <div
      data-tauri-drag-region
      class="flex flex-1 items-center justify-center px-4"
    >
      <AppActiveDownloads />
      <InstanceBlockedFilesModal
        v-if="awaiting"
        :key="awaiting.instanceId"
        :install="awaiting"
      />
      <ImportOpenedFile />
    </div>

    <div class="flex items-stretch">
      <AppErrorCenter />

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
  </header>
</template>
