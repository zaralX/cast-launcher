<script setup lang="ts">
import LoadingScreen from "~/components/LoadingScreen.vue";

const loading = ref(true)
const steps = ["Ожидание", "Готово!"]
const currentStep = ref()

onMounted(() => {
  currentStep.value = 1
  loading.value = false
})
</script>

<template>
<div class="">
  <LoadingScreen v-model="currentStep" :steps="steps" v-if="loading" />
  <div v-else>
    <UButton>Button</UButton>
  </div>
</div>
</template>

<style scoped>

</style>