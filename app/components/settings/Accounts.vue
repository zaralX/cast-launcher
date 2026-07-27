<script setup lang="ts">
import {storeToRefs} from "pinia";
import {useAccountStore} from "~/stores/account";

const accountStore = useAccountStore();
const { accountConfig } = storeToRefs(accountStore)

const offlineNickname = ref("")
const loggingIn = ref(false)

const createOfflineAccount = async () => {
  const name = offlineNickname.value.trim()
  if (!name) return

  accountConfig.value!.accounts.push({
    type: 'offline',
    name
  })
  accountConfig.value!.selected = accountConfig.value!.accounts.length - 1

  await safeRun(() => accountStore.updateConfig(accountConfig.value!))
  offlineNickname.value = ""
}

const createMicrosoftAccount = async () => {
  loggingIn.value = true
  await safeRun(() => accountStore.microsoftLogin(), {code: "AUTH_FAILED"})
  loggingIn.value = false
}

const selectAccount = (index: number) => safeRun(() => accountStore.selectAccount(index))
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
           @click="selectAccount(i)">
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
        <UButton icon="i-lucide-plus" :loading="loggingIn" @click="createMicrosoftAccount">Microsoft аккаунт</UButton>
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