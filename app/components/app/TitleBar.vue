<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'

const instanceStore = useInstanceStore()

const awaiting = computed(() => instanceStore.installs.find(install => install.awaitingFiles) ?? null)

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
  <header
    data-tauri-drag-region
    class="relative z-40 flex h-10 shrink-0 items-stretch bg-frame"
  >
    <AppLogo>
      <span
        v-if="version"
        class="font-mono text-micro text-fg-faint"
      >{{ version }}</span>
    </AppLogo>

    <div
      data-tauri-drag-region
      class="flex-1"
    />

    <AppUpdateNotice />
    <AppActiveDownloads />

    <AppWindowControls>
      <AppErrorCenter />
    </AppWindowControls>

    <InstanceBlockedFilesModal
      v-if="awaiting"
      :key="awaiting.instanceId"
      :install="awaiting"
    />
    <ImportOpenedFile />
  </header>
</template>
