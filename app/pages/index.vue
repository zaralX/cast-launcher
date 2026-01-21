<script setup lang="ts">
import LoadingScreen from "~/components/LoadingScreen.vue";
import {invoke} from "@tauri-apps/api/core";
import {useAppStore} from "~/stores/app";
import {useInstanceStore} from "~/stores/instance";
import {check} from "@tauri-apps/plugin-updater";

const loading = ref(true)
const steps = ["Ожидание", "Получение конфигураций", "Проверка обновлений", "Получение сборок zaralX", "Подготовка instances", "Готово!"]
const currentStep = ref()
const appStore = useAppStore();
const accountStore = useAccountStore();
const instanceStore = useInstanceStore();

onMounted(() => {
  setTimeout(async () => {
    currentStep.value = 1
    await appStore.loadConfig()
    await accountStore.loadConfig()
    currentStep.value += 1

    try {
      if (appStore.config!.launcher.auto_update && await check({ timeout: 15000 })) {
        await appStore.updateApp()
      }
    } catch (e) {
      console.error("Failed to auto update app", e)
    }
    currentStep.value += 1

    try {
      await appStore.loadMyPacks()
    } catch (e) {
      console.error("Failed to load myPacks", e)
    }
    currentStep.value += 1

    await instanceStore.initInstances()
    currentStep.value += 1

    loading.value = false
    navigateTo("/main")
    invoke("greet", {name: "Cast Launcher"}).then((res) => console.log(res));
  }, 2000)
})
</script>

<template>
<div class="">
  <LoadingScreen v-model="currentStep" :steps="steps" />
</div>
</template>

<style scoped>

</style>