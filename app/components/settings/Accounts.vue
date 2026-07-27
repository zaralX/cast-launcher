<script setup lang="ts">
import {storeToRefs} from "pinia";
import {useAccountStore} from "~/stores/account";

const accountStore = useAccountStore();
const {accountConfig} = storeToRefs(accountStore)

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
  <SettingsPanel
      index="02"
      title="Аккаунты"
      description="С каким профилем запускается игра."
      icon="i-lucide-user-round"
  >
    <div class="space-y-7">
      <ul v-if="accountConfig?.accounts?.length" class="border-t border-line">
        <li
            v-for="(account, i) in accountConfig!.accounts"
            :key="`${account.type}-${account.name}-${i}`"
            class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 pr-1 transition-colors duration-300 hover:bg-ink-700"
            @click="selectAccount(i)"
        >
          <!-- Активный профиль отмечен кантом, а не рамкой вокруг всей строки -->
          <span
              class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
              :class="accountConfig?.selected === i ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
          />

          <NuxtImg
              :src="`https://assets.zaralx.ru/api/v1/minecraft/vanilla/player/face/${account.name}/full`"
              class="size-8 shrink-0 transition-transform duration-500 ease-deck group-hover:scale-105"
              :alt="account.name"
          />

          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] text-fg">{{ account.name }}</p>
            <p class="mt-1 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ account.type === 'microsoft' ? 'Microsoft' : 'Оффлайн' }}
            </p>
          </div>

          <span
              v-if="accountConfig?.selected === i"
              class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-acid"
          >
            Активен
          </span>

          <UIcon
              :name="account.type === 'microsoft' ? 'mdi:microsoft' : 'i-lucide-globe'"
              class="size-4 shrink-0 text-fg-faint"
          />
        </li>
      </ul>

      <p v-else class="border-y border-line py-6 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
        Ни одного аккаунта
      </p>

      <div class="grid gap-4 sm:grid-cols-2">
        <button
            type="button"
            class="group/act relative flex h-10 items-center justify-center overflow-hidden border border-line font-mono text-[10px] uppercase tracking-[0.18em] text-fg transition-colors duration-300 hover:border-acid hover:text-on-acid"
            @click="createMicrosoftAccount"
        >
          <span
              class="absolute inset-0 origin-left scale-x-0 bg-acid transition-transform duration-500 ease-deck group-hover/act:scale-x-100"
              aria-hidden="true"
          />
          <span class="relative flex items-center gap-2">
            <UIcon
                :name="loggingIn ? 'i-lucide-loader-circle' : 'mdi:microsoft'"
                class="size-3.5"
                :class="loggingIn ? 'animate-spin' : ''"
            />
            Microsoft
          </span>
        </button>

        <UPopover mode="hover">
          <button
              type="button"
              class="flex h-10 w-full items-center justify-center gap-2 border border-line font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted transition-colors duration-300 hover:border-line-strong hover:text-fg"
          >
            <UIcon name="i-lucide-globe" class="size-3.5"/>
            Оффлайн
          </button>

          <template #content>
            <div class="w-64 space-y-4 p-5">
              <SettingsField label="Никнейм">
                <UInput v-model="offlineNickname" placeholder="nickname" class="w-full"/>
              </SettingsField>
              <button
                  type="button"
                  class="group/act relative flex h-9 w-full items-center justify-center overflow-hidden border border-line font-mono text-[10px] uppercase tracking-[0.18em] text-fg transition-colors duration-300 hover:border-acid hover:text-on-acid disabled:pointer-events-none disabled:opacity-30"
                  :disabled="!offlineNickname.trim()"
                  @click="createOfflineAccount"
              >
                <span
                    class="absolute inset-0 origin-left scale-x-0 bg-acid transition-transform duration-500 ease-deck group-hover/act:scale-x-100"
                    aria-hidden="true"
                />
                <span class="relative">Добавить</span>
              </button>
            </div>
          </template>
        </UPopover>
      </div>
    </div>
  </SettingsPanel>
</template>
