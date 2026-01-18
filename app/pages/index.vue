<script setup lang="ts">
import LoadingScreen from "~/components/LoadingScreen.vue";

const loading = ref(true)
const steps = ["Ожидание", "Готово!"]
const currentStep = ref()

onMounted(() => {
  setTimeout(() => {
    currentStep.value = 1
    loading.value = false
    navigateTo("/main")
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