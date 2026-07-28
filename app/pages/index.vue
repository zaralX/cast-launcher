<script setup lang="ts">
import LoadingScreen from "~/components/LoadingScreen.vue";
import {useAppStore} from "~/stores/app";

const loading = ref(true)
const steps = ["Ожидание", "Подключение к лаунчеру", "Проверка обновлений", "Получение сборок zaralX", "Готово!"]
const currentStep = ref(0)
const appStore = useAppStore();

onMounted(async () => {
  currentStep.value = 1

  await safeRun(() => useLauncherEvents(), {code: "CONFIG_ERROR"})
  currentStep.value += 1

  if (appStore.config?.launcher?.auto_update) {
    await safeRun(() => appStore.updateApp(), {code: "UPDATE_FAILED"})
  }
  currentStep.value += 1

  await safeRun(() => appStore.loadMyPacks(), {code: "NETWORK"})
  currentStep.value += 1

  loading.value = false
  navigateTo("/main")
})
</script>

<template>
  <div class="h-screen w-full">
    <LoadingScreen v-model="currentStep" :steps="steps"/>
  </div>
</template>
