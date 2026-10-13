<script setup lang="ts">
const accountStore = useAccountStore()
const { accountConfig, loggingIn } = storeToRefs(accountStore)

const nickname = ref('')
const adding = ref(false)

const accounts = computed(() => accountConfig.value?.accounts ?? [])

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
        variant="primary"
        size="lg"
        icon="simple-icons:microsoft"
        class="w-full"
        :loading="loggingIn"
        @click="loginMicrosoft"
      >
        Microsoft
      </Button>

      <div class="flex items-center gap-3">
        <Separator class="flex-1" />
        <span class="text-caption text-fg-faint">{{ $t('settings.accounts.offline') }}</span>
        <Separator class="flex-1" />
      </div>

      <div class="flex gap-2">
        <Input
          v-model="nickname"
          :placeholder="$t('settings.accounts.nickname')"
          :aria-label="$t('settings.accounts.nickname')"
          size="lg"
          @keydown.enter="addOffline"
        />

        <Button
          size="lg"
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
        class="divide-y divide-line overflow-hidden rounded-md bg-panel ring-1 ring-card-line ring-inset"
      >
        <KitListItem
          v-for="(account, i) in accounts"
          :key="`${account.type}-${account.name}-${i}`"
          :active="accountConfig?.selected === i"
          :active-label="$t('settings.accounts.active')"
          class="rounded-none px-4"
          @select="select(i)"
        >
          <template #leading>
            <KitAvatar :name="account.name" />
          </template>

          <p class="truncate font-medium text-fg">
            {{ account.name }}
          </p>
          <p class="truncate text-caption text-fg-faint">
            {{ account.type === 'microsoft' ? 'Microsoft' : $t('settings.accounts.offline') }}
          </p>
        </KitListItem>
      </ul>
    </div>
  </OnboardingPane>
</template>
