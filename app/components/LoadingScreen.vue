<script setup lang="ts">
const model = defineModel<number>()
const props = defineProps<{
  steps: string[]
}>()
</script>

<template>
<div data-tauri-drag-region class="flex items-center justify-center h-screen">
  <div class="w-96 text-center space-y-4">
    <p>Подготовка лаунчера</p>
    <UProgress v-model="model" :max="steps" />
  </div>
</div>
</template>

<style scoped>

</style>