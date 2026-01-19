<script setup lang="ts">
import CreateInstanceModalBody from "~/components/CreateInstanceModalBody.vue";

definePageMeta({
  layout: "main"
})

const instanceStore = useInstanceStore()
const {installInstance, runInstance} = instanceStore
const {runningClients, instances} = storeToRefs(instanceStore)
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
          :title="instance.name"
          :description="instance.description"
          orientation="vertical"
          class="w-full"
          v-for="instance in instances"
      >
        <UButton v-if="!instance.installed" icon="i-lucide-download" @click="installInstance(instance.id)" variant="subtle" :loading="instance.installing">Загрузить</UButton>
        <UButton v-else icon="i-lucide-play" @click="runInstance(instance.id)" :loading="!!runningClients.find(c => c.instance.id == instance.id)">Играть</UButton>
      </UPageCard>
      <UModal title="Создание сборки">
        <UButton label="Создать" color="neutral" variant="subtle" />

        <template #body>
          <CreateInstanceModalBody />
        </template>
      </UModal>
    </div>
  </div>
</template>

<style scoped>

</style>