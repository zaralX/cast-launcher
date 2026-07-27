<script setup lang="ts">
import LoadingScreen from "~/components/LoadingScreen.vue";
import {useAppStore} from "~/stores/app";
import {useInstanceStore} from "~/stores/instance";
import {check} from "@tauri-apps/plugin-updater";

const loading = ref(true)
const steps = ["Ожидание", "Получение конфигураций", "Проверка обновлений", "Получение сборок zaralX", "Подготовка instances", "Готово!"]
const currentStep = ref()
const appStore = useAppStore();
const accountStore = useAccountStore();
const instanceStore = useInstanceStore();

onMounted(async () => {
  currentStep.value = 1

  await safeRun(() => appStore.loadConfig(), {code: "CONFIG_ERROR"})
  await safeRun(() => accountStore.loadConfig(), {code: "CONFIG_ERROR"})
  currentStep.value += 1

  if (appStore.config?.launcher?.auto_update) {
    await safeRun(async () => {
      if (await check({timeout: 15000})) await appStore.updateApp()
    }, {code: "UPDATE_FAILED"})
  }
  currentStep.value += 1

  await safeRun(() => appStore.loadMyPacks(), {code: "NETWORK"})
  currentStep.value += 1

  await safeRun(() => instanceStore.initInstances(), {code: "FS_ERROR"})
  currentStep.value += 1

  loading.value = false
  navigateTo("/main")
})
</script>

<template>
<div class="h-screen w-full">
  <LoadingScreen v-model="currentStep" :steps="steps" />
</div>
</template>

<style scoped>

</style>