<script setup lang="ts">
import {type InstallerProgress, isTerminalStage} from "~/types/instance";

const instanceStore = useInstanceStore()
const {currentInstaller} = storeToRefs(instanceStore)

const latestMessage = ref<InstallerProgress | null>()
const globalDownloading = ref(0)

let unsubscribeInstaller: (() => void) | null = null

watch(currentInstaller, (newValue) => {
  unsubscribeInstaller?.()
  unsubscribeInstaller = null

  if (!newValue) return;

  globalDownloading.value = 0

  unsubscribeInstaller = newValue.onProgress((p: InstallerProgress) => {
    p.progress = (p.progress ?? 0) * 100

    if (p.type == "global") {
      globalDownloading.value = p.progress
    }

    if (isTerminalStage(p.stage)) {
      latestMessage.value = null
      unsubscribeInstaller?.()
      unsubscribeInstaller = null
      return
    }

    latestMessage.value = p
  })
})

onUnmounted(() => unsubscribeInstaller?.())
</script>

<template>
  <UModal v-if="latestMessage" title="Загрузка файлов" class="ml-auto mr-0">
    <div class="flex items-center px-2 bg-zinc-800/50 border border-zinc-800 rounded-lg min-w-64 ml-0! text-sm py-0.5 hover:bg-zinc-800/75 cursor-pointer select-none">
      <div class="w-2 h-2 rounded-full bg-sky-500 mr-2">
        <div class="w-2 h-2 rounded-full bg-sky-500 animate-ping">

        </div>
      </div>
      <p class="text-sky-400">{{latestMessage.message}}</p>
    </div>

    <template #body>
      <p>Общий</p>
      <UProgress v-model="globalDownloading"/>
      <p>Сообщение {{ latestMessage.message }}</p>
      <UProgress v-model="latestMessage.progress"/>
    </template>
  </UModal>
</template>

<style scoped>

</style>