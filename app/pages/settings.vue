<script setup lang="ts">
import { useAppStore } from "~/stores/app";
import { storeToRefs } from "pinia";

definePageMeta({
  layout: "main"
});

const store = useAppStore();
const { config } = storeToRefs(store);

const toast = useToast()

async function saveConfig() {
  if (!config.value) return;
  try {
    await store.updateConfig(config.value)
    toast.add({
      title: 'Настройки сохранены',
      color: 'success',
      icon: 'i-lucide-save'
    })
  } catch (e) {
    toast.add({
      title: 'Произошла ошибка',
      description: 'Не получилось сохранить настройки',
      color: 'error',
      icon: 'i-lucide-save'
    })
  }
}
</script>

<template>
  <div class="p-4 gap-6 w-full grid lg:grid-cols-2">
    <!-- Launcher -->
    <SettingsLauncher v-model="config" />

    <!-- Accounts -->
    <SettingsAccounts v-model="config" />

    <!-- Java -->
    <UPageCard
        title="Java"
        description="Параметры виртуальной машины Java."
        variant="soft"
    >
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <UFormField  label="Путь к Java">
          <UInput
              v-model="config!.java.java_path"
              placeholder="/path/to/java"
          />
        </UFormField >

        <UFormField  label="Минимум RAM (MB)">
          <UInput
              type="number"
              v-model="config!.java.min_ram"
          />
        </UFormField >

        <UFormField  label="Максимум RAM (MB)">
          <UInput
              type="number"
              v-model="config!.java.max_ram"
          />
        </UFormField >
      </div>
    </UPageCard>

    <!-- Actions -->
    <div class="flex justify-end">
      <UButton color="primary" @click="saveConfig">
        Сохранить
      </UButton>
    </div>
  </div>
</template>

<style scoped>
</style>
