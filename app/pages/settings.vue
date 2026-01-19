<script setup lang="ts">
import { useAppStore } from "~/stores/app";
import { storeToRefs } from "pinia";
import { ru } from "@nuxt/ui/locale";
import {useAccountStore} from "~/stores/account";

definePageMeta({
  layout: "main"
});

const store = useAppStore();
const accountStore = useAccountStore();
const { config: accountConfig } = storeToRefs(accountStore)
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

const offlineNickname = ref("")

const createOfflineAccount = () => {
  accountConfig.value!.accounts.push({
    type: 'offline',
    name: offlineNickname.value
  })
  accountConfig.value!.selected = accountConfig.value!.accounts.length - 1
  accountStore.updateConfig(accountConfig.value)
}
</script>

<template>
  <div class="p-4 gap-6 w-full grid grid-cols-2">
    <!-- Launcher -->
    <UPageCard
        title="Лаунчер"
        description="Базовые настройки лаунчера."
        variant="soft"
    >
      <div class="space-y-4">
        <div>
          <label class="text-sm mb-1 block">Язык</label>
          <ULocaleSelect
              v-model="config!.launcher.language"
              :locales="[ru]"
              class="w-48"
          />
        </div>

        <div>
          <label class="text-sm mb-1 block">Тема</label>
          <UColorModeSelect />
        </div>

        <div>
          <UFormField label="Файлы лаунчера">
            <UInput
                v-model="config!.launcher.dir"
                placeholder="/path/to/launcher"
            />
          </UFormField >
        </div>
      </div>
    </UPageCard>

    <!-- Accounts -->
    <UPageCard
        title="Аккаунты"
        description="Это ваши аккаунты."
        variant="soft"
    >
      <div class="space-y-4">
        <UPageCard
            v-for="(account, i) in accountConfig!.accounts"
            :title="account.name"
            :description="`type: ${account.type}`"
            variant="soft"
            :highlight="!!(accountConfig?.selected == i)"
        />
        <div class="grid grid-cols-2 gap-4">
          <UButton icon="i-lucide-plus" disabled>Microsoft аккаунт</UButton>
          <UPopover mode="hover">
            <UButton icon="i-lucide-plus">Оффлайн аккаунт</UButton>

            <template #content>
              <div class="p-2">
                <UFormField  label="Никнейм">
                  <UInput
                      v-model="offlineNickname"
                      placeholder="nickname"
                  />
                </UFormField >
                <UButton icon="i-lucide-plus" @click="createOfflineAccount">Создать оффлайн аккаунт</UButton>
              </div>
            </template>
          </UPopover>
        </div>
      </div>
    </UPageCard>

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
