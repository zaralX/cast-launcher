<script setup lang="ts">
const instancesStore = useInstanceStore()

const createInstance = async () => {
  await instancesStore.createInstance({
    id: "dada-baby",
    name: "yee boy",
    description: "okak",
    version: 1
  })
}
</script>

<template>
<div>
  <UButton @click="createInstance">Создать</UButton>
</div>
</template>

<style scoped>

</style>