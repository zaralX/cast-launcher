<script setup lang="ts">
const accountStore = useAccountStore()
const { accountConfig, loggingIn } = storeToRefs(accountStore)

const nickname = ref('')
const adding = ref(false)

const accounts = computed(() => accountConfig.value?.accounts ?? [])

const faceOf = (name: string) =>
  `https://assets.zaralx.ru/api/v2/minecraft/players/${encodeURIComponent(name)}/face`

async function addOffline() {
  const name = nickname.value.trim()
  if (!name || adding.value) return

  adding.value = true
  await safeRun(() => accountStore.addOfflineAccount(name))
  adding.value = false

  nickname.value = ''
}

const loginMicrosoft = () =>
  safeRun(() => accountStore.microsoftLogin(), { code: 'AUTH_FAILED' })

const select = (index: number) => safeRun(() => accountStore.selectAccount(index))
</script>

<template>
  <OnboardingPane
    :title="$t('onboarding.account.title')"
    width="narrow"
  >
    <div class="space-y-6">
      <Button
        size="xl"
        icon="simple-icons:microsoft"
        class="h-12 w-full"
        :loading="loggingIn"
        @click="loginMicrosoft"
      >
        Microsoft
      </Button>

      <div class="flex items-center gap-4">
        <Separator class="flex-1" />
        <span class="font-mono text-micro uppercase tracking-caps text-fg-faint">
          {{ $t('settings.accounts.offline') }}
        </span>
        <Separator class="flex-1" />
      </div>

      <div class="flex gap-2">
        <Input
          v-model="nickname"
          placeholder="nickname"
          size="lg"
          font="mono"
          @keydown.enter="addOffline"
        />

        <Button
          icon="i-lucide-plus"
          :loading="adding"
          :disabled="!nickname.trim()"
          @click="addOffline"
        >
          {{ $t('common.add') }}
        </Button>
      </div>

      <ul
        v-if="accounts.length"
        class="border-t border-line"
      >
        <KitListItem
          v-for="(account, i) in accounts"
          :key="`${account.type}-${account.name}-${i}`"
          :active="accountConfig?.selected === i"
          :active-label="$t('settings.accounts.active')"
          class="pr-4"
          @select="select(i)"
        >
          <template #leading>
            <img
              :src="faceOf(account.name)"
              class="size-8 shrink-0"
              :alt="account.name"
              @error="fallbackFace"
            >
          </template>

          <p class="truncate text-title text-fg">
            {{ account.name }}
          </p>
          <p class="mt-1 font-mono text-micro uppercase tracking-caps text-fg-faint">
            {{ account.type === 'microsoft' ? 'Microsoft' : $t('settings.accounts.offline') }}
          </p>
        </KitListItem>
      </ul>
    </div>
  </OnboardingPane>
</template>
