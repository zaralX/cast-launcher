<script setup lang="ts">
import LoadingScreen from "~/components/LoadingScreen.vue";
import {invoke} from "@tauri-apps/api/core";
const loading = ref(true)
const steps = ["Ожидание", "Готово!"]
const currentStep = ref()

onMounted(() => {
  setTimeout(() => {
    currentStep.value = 1
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