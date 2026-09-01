<script setup lang="ts">
import {storeToRefs} from "pinia"
import {useAccountStore} from "~/stores/account"

const accountStore = useAccountStore()
const {accountConfig, loggingIn} = storeToRefs(accountStore)

const nickname = ref("")
const preview = ref("")
const adding = ref(false)

let timer: ReturnType<typeof setTimeout> | null = null

watch(nickname, value => {
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => {
    preview.value = value.trim()
  }, 350)
})

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})

const faceOf = (name: string) =>
    `https://assets.zaralx.ru/api/v1/minecraft/vanilla/player/face/${encodeURIComponent(name)}/full`

const accounts = computed(() => accountConfig.value?.accounts ?? [])

async function addOffline() {
  const name = nickname.value.trim()
  if (!name || adding.value) return

  adding.value = true
  await safeRun(() => accountStore.addOfflineAccount(name))
  adding.value = false

  nickname.value = ""
  preview.value = ""
}

const loginMicrosoft = () =>
    safeRun(() => accountStore.microsoftLogin(), {code: "AUTH_FAILED"})

const select = (index: number) => safeRun(() => accountStore.selectAccount(index))
</script>

<template>
  <OnboardingPane
      index="04 / 06"
      :title="$t('onboarding.account.title')"
  >
    <div class="grid gap-8 lg:grid-cols-[minmax(0,1fr)_17rem]">
      <div class="space-y-5">
        <button
            type="button"
            class="group relative flex w-full cursor-pointer items-center gap-5 overflow-hidden border border-line bg-ink-800/50 px-6 py-5 text-left transition-colors duration-300 hover:border-acid/60 hover:bg-ink-800 disabled:cursor-wait"
            :disabled="loggingIn"
            @click="loginMicrosoft"
        >
          <span
              class="pointer-events-none absolute inset-0 -translate-x-full bg-acid/[0.07] transition-transform duration-700 ease-deck group-hover:translate-x-0"
          />

          <UIcon
              :name="loggingIn ? 'i-lucide-loader-circle' : 'simple-icons:microsoft'"
              class="relative size-6 shrink-0 text-acid"
              :class="loggingIn ? 'animate-spin' : ''"
          />

          <span class="relative min-w-0 flex-1">
            <span class="block font-mono text-[11px] uppercase tracking-[0.2em] text-fg">Microsoft</span>
          </span>

          <UIcon
              name="i-lucide-arrow-right"
              class="relative size-4 shrink-0 text-fg-faint transition-transform duration-500 ease-deck group-hover:translate-x-1 group-hover:text-acid"
          />
        </button>

        <div class="border border-line bg-ink-800/50 px-6 py-5">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-globe" class="size-4 shrink-0 text-fg-faint"/>
            <p class="font-mono text-[11px] uppercase tracking-[0.2em] text-fg">
              {{ $t('settings.accounts.offline') }}
            </p>
          </div>

          <div class="mt-4 flex gap-2">
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
        </div>

        <TransitionGroup
            v-if="accounts.length"
            tag="ul"
            name="account"
            class="border-t border-line"
        >
          <li
              v-for="(account, i) in accounts"
              :key="`${account.type}-${account.name}-${i}`"
              class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 pr-4 transition-colors duration-300 hover:bg-ink-700"
              @click="select(i)"
          >
            <span
                class="absolute inset-y-0 left-0 w-[2px] origin-top bg-acid transition-transform duration-500 ease-deck"
                :class="accountConfig?.selected === i ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
            />

            <img
                :src="faceOf(account.name)"
                class="size-8 shrink-0 transition-transform duration-500 ease-deck group-hover:scale-105"
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
        </TransitionGroup>
      </div>

      <aside class="flex flex-col items-center justify-center border border-line bg-ink-800/40 px-6 py-8">
        <div class="relative grid size-32 place-items-center">
          <span class="absolute inset-0 animate-orbit">
            <span class="absolute -top-1 left-1/2 size-1.5 -translate-x-1/2 bg-acid"/>
          </span>
          <span class="absolute inset-2 border border-line"/>
          <span class="absolute inset-6 border border-line/60"/>

          <Transition name="face" mode="out-in">
            <img
                v-if="preview"
                :key="preview"
                :src="faceOf(preview)"
                class="relative size-16"
                alt=""
                @error="fallbackFace"
            />
            <UIcon
                v-else
                key="empty"
                name="i-lucide-user-round"
                class="relative size-10 text-fg-faint/50"
            />
          </Transition>
        </div>

        <p class="mt-6 text-center font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
          {{ preview || $t('onboarding.account.no_name') }}
        </p>

        <p
            class="mt-4 text-center font-mono text-[10px] leading-relaxed"
            :class="accounts.length ? 'text-acid' : 'text-amber-400'"
        >
          {{ accounts.length ? $t('onboarding.account.ready') : $t('onboarding.account.required') }}
        </p>
      </aside>
    </div>
  </OnboardingPane>
</template>

<style scoped>
.face-enter-active,
.face-leave-active {
  transition: opacity 0.25s ease, transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.face-enter-from,
.face-leave-to {
  opacity: 0;
  transform: scale(0.85);
}

.account-enter-active,
.account-leave-active {
  transition: opacity 0.35s ease, transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}

.account-enter-from {
  opacity: 0;
  transform: translateX(-12px);
}

.account-leave-to {
  opacity: 0;
  transform: translateX(12px);
}
</style>
