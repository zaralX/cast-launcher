<script setup lang="ts">
import {storeToRefs} from "pinia"
import {useAccountStore} from "~/stores/account"

const accountStore = useAccountStore()
const {accountConfig, loggingIn} = storeToRefs(accountStore)

const nickname = ref("")
const adding = ref(false)

const accounts = computed(() => accountConfig.value?.accounts ?? [])

const faceOf = (name: string) =>
    `https://assets.zaralx.ru/api/v1/minecraft/vanilla/player/face/${encodeURIComponent(name)}/full`

async function addOffline() {
  const name = nickname.value.trim()
  if (!name || adding.value) return

  adding.value = true
  await safeRun(() => accountStore.addOfflineAccount(name))
  adding.value = false

  nickname.value = ""
}

const loginMicrosoft = () =>
    safeRun(() => accountStore.microsoftLogin(), {code: "AUTH_FAILED"})

const select = (index: number) => safeRun(() => accountStore.selectAccount(index))
</script>

<template>
  <OnboardingPane :title="$t('onboarding.account.title')" width="narrow">
    <div class="space-y-6">
      <AppButton
          block
          class="h-12 tracking-[0.2em]"
          icon="simple-icons:microsoft"
          :loading="loggingIn"
          @click="loginMicrosoft"
      >
        Microsoft
      </AppButton>

      <div class="flex items-center gap-4">
        <span class="h-px flex-1 bg-line"/>
        <span class="font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">
          {{ $t('settings.accounts.offline') }}
        </span>
        <span class="h-px flex-1 bg-line"/>
      </div>

      <div class="flex gap-2">
        <UInput
            v-model="nickname"
            placeholder="nickname"
            class="w-full"
            :ui="{ base: 'font-mono text-[12px]' }"
            @keydown.enter="addOffline"
        />

        <AppButton
            class="h-9 shrink-0 px-4 text-[10px] tracking-[0.18em]"
            icon="i-lucide-plus"
            :loading="adding"
            :disabled="!nickname.trim()"
            @click="addOffline"
        >
          {{ $t('common.add') }}
        </AppButton>
      </div>

      <ul v-if="accounts.length" class="border-t border-line">
        <li
            v-for="(account, i) in accounts"
            :key="`${account.type}-${account.name}-${i}`"
            class="group relative flex cursor-pointer items-center gap-4 border-b border-line px-4 py-3.5 transition-colors duration-300 hover:bg-ink-700"
            @click="select(i)"
        >
          <span
              class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
              :class="accountConfig?.selected === i ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
          />

          <img
              :src="faceOf(account.name)"
              class="size-8 shrink-0"
              :alt="account.name"
              @error="fallbackFace"
          />

          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] text-fg">{{ account.name }}</p>
            <p class="mt-1 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ account.type === 'microsoft' ? 'Microsoft' : $t('settings.accounts.offline') }}
            </p>
          </div>

          <span
              v-if="accountConfig?.selected === i"
              class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-acid"
          >
            {{ $t('settings.accounts.active') }}
          </span>
        </li>
      </ul>
    </div>
  </OnboardingPane>
</template>
