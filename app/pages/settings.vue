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
    <SettingsAccounts />

    <!-- Java -->
    <SettingsJava v-model="config" />

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
