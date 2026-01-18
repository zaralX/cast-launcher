<script setup lang="ts">
import LoadingScreen from "~/components/LoadingScreen.vue";
import {invoke} from "@tauri-apps/api/core";
import {useAppStore} from "~/stores/app";

const loading = ref(true)
const steps = ["Ожидание", "Получение конфигурации", "Готово!"]
const currentStep = ref()
const store = useAppStore();

onMounted(() => {
  setTimeout(async () => {
    currentStep.value = 1
    await store.loadConfig()
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