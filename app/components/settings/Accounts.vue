<script setup lang="ts">
import {storeToRefs} from "pinia";
import {useAccountStore} from "~/stores/account";

const accountStore = useAccountStore();
const { accountConfig } = storeToRefs(accountStore)

const offlineNickname = ref("")

const createOfflineAccount = () => {
  accountConfig.value!.accounts.push({
    type: 'offline',
    name: offlineNickname.value
  })
  accountConfig.value!.selected = accountConfig.value!.accounts.length - 1
  accountStore.updateConfig(accountConfig.value!)
}

const createMicrosoftAccount = async () => {
  await accountStore.microsoftLogin()
}
</script>

<template>
  <UPageCard
      title="Аккаунты"
      description="Это ваши аккаунты."
      variant="soft"
  >
    <div class="space-y-4">
      <div v-for="(account, i) in accountConfig!.accounts" class="
      bg-zinc-800/50 rounded-lg p-2 flex items-center gap-2
      border-2 hover:bg-zinc-800 transition-all cursor-pointer"
           :class="accountConfig?.selected == i ? 'border-sky-500' : 'border-transparent'"
           @click="accountStore.selectAccount(i)">
        <div>
          <NuxtImg :src="`https://assets.zaralx.ru/api/v1/minecraft/vanilla/player/face/${account.name}/full`" class="w-8 h-8" />
        </div>
        <div class="flex-1">
          <p>{{ account.name }}</p>
        </div>
        <Icon v-if="account.type == 'microsoft'" name="mdi:microsoft" size="24" />
        <Icon v-else name="mdi:globe-x" size="24" />
      </div>
      <div class="grid grid-cols-2 gap-4">
        <UButton icon="i-lucide-plus" @click="createMicrosoftAccount">Microsoft аккаунт</UButton>
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
</template>

<style scoped>

</style>