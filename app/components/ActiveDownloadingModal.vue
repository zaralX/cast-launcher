<script setup lang="ts">
import type {InstallerProgress} from "~/types/instance";

const instanceStore = useInstanceStore()
const {currentInstaller} = storeToRefs(instanceStore)

const latestMessage = ref({})
const globalDownloading = ref(0)

watch(currentInstaller, (newValue) => {
  if (!newValue) return;
  const unsubscribeInstaller = newValue.onProgress((p: InstallerProgress) => {
    p.progress = (p.progress ?? 0) * 100
    latestMessage.value = p
    if (p.type == "global") {
      globalDownloading.value = p.progress
    }
    if (p.stage == "finished" || p.stage == "aborted") {
      unsubscribeInstaller()
    }
  })
})
</script>

<template>
  <UModal title="Загрузка файлов" class="ml-auto mr-0">
    <UButton label="Открыть" color="neutral" size="xl" variant="subtle" />

    <template #body>
      <p>Общий</p>
      <UProgress v-model="globalDownloading" />
      <p>Сообщение {{latestMessage.message}}</p>
      <UProgress v-model="latestMessage.progress" />
    </template>
  </UModal>
</template>

<style scoped>

</style>