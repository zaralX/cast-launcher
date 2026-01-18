<script setup lang="ts">
definePageMeta({
  layout: "main"
})
</script>

<template>
  <div class="p-4 space-y-2 w-full">
    <p class="font-unbounded">Сборки от zaralX</p>
    <div class="grid grid-cols-3 w-full gap-4">
      <UPageCard
          title="Random Mods"
          description="Каждый день добавляется случайный мод"
          orientation="vertical"
          spotlight
          spotlight-color="primary"
          class="w-full"
          v-for="i in 3"
      >
        <UButton icon="i-lucide-download" variant="subtle">Загрузить</UButton>
      </UPageCard>
    </div>
    <p class="font-unbounded">Ваши сборки</p>
    <div class="grid grid-cols-5 w-full gap-4">
      <UPageCard
          title="Random Mods"
          description="Каждый день добавляется случайный мод"
          orientation="vertical"
          class="w-full"
          v-for="i in 5"
      >
        <UButton icon="i-lucide-play">Играть</UButton>
      </UPageCard>
    </div>
  </div>
</template>

<style scoped>

</style>