<script setup lang="ts">
import type { Account } from '~/types/account'

const { t } = useI18n()
const accountStore = useAccountStore()
const { accountConfig, loggingIn } = storeToRefs(accountStore)
const toast = useAppToast()

const offlineNickname = ref('')
const removeTarget = ref<Account | null>(null)
const removing = ref(false)

const createOfflineAccount = async () => {
  const name = offlineNickname.value.trim()
  if (!name) return

  await safeRun(() => accountStore.addOfflineAccount(name))
  offlineNickname.value = ''
}

const createMicrosoftAccount = () => safeRun(() => accountStore.microsoftLogin(), { code: 'AUTH_FAILED' })

const selectAccount = (index: number) => safeRun(() => accountStore.selectAccount(index))

async function confirmRemove() {
  const target = removeTarget.value
  if (!target?.uuid || removing.value) return

  removing.value = true
  const result = await attempt(() => accountStore.removeAccount(target.uuid!))
  removing.value = false

  if (!result.ok) return

  removeTarget.value = null

  toast.add({
    title: t('settings.accounts.removed', { name: target.name }),
    color: 'success',
    icon: 'i-lucide-trash-2',
  })
}
</script>

<template>
  <KitPanel
    index="02"
    :title="$t('settings.accounts.title')"
    icon="i-lucide-user-round"
  >
    <div class="space-y-7">
      <ul
        v-if="accountConfig?.accounts?.length"
        class="border-t border-line"
      >
        <KitListItem
          v-for="(account, i) in accountConfig!.accounts"
          :key="`${account.type}-${account.name}-${i}`"
          :active="accountConfig?.selected === i"
          :active-label="$t('settings.accounts.active')"
          class="pr-4"
          @select="selectAccount(i)"
        >
          <template #leading>
            <img
              :src="`https://assets.zaralx.ru/api/v2/minecraft/players/${account.name}/face`"
              class="size-8 shrink-0 transition-transform duration-500 ease-deck group-hover:scale-105"
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

          <template #trailing>
            <Button
              variant="quiet-danger"
              size="icon-sm"
              icon="i-lucide-trash-2"
              :aria-label="$t('settings.accounts.remove')"
              class="text-fg-faint"
              :disabled="!account.uuid"
              @click.stop="removeTarget = account"
            />
          </template>
        </KitListItem>
      </ul>

      <p
        v-else
        class="border-y border-line py-6 text-center font-mono text-label uppercase tracking-caps text-fg-faint"
      >
        {{ $t('settings.accounts.empty') }}
      </p>

      <div class="grid grid-cols-2 gap-4">
        <Button
          size="lg"
          icon="simple-icons:microsoft"
          class="w-full text-label"
          :loading="loggingIn"
          @click="createMicrosoftAccount"
        >
          Microsoft
        </Button>

        <HoverCard>
          <HoverCardTrigger as-child>
            <Button
              variant="outline"
              size="lg"
              icon="i-lucide-globe"
              class="w-full text-label"
            >
              {{ $t('settings.accounts.offline') }}
            </Button>
          </HoverCardTrigger>

          <HoverCardContent class="w-64 space-y-4 p-5">
            <KitField
              :label="$t('settings.accounts.nickname')"
              for="offline-nickname"
            >
              <Input
                id="offline-nickname"
                v-model="offlineNickname"
                placeholder="nickname"
                @keydown.enter="createOfflineAccount"
              />
            </KitField>
            <Button
              class="w-full"
              :disabled="!offlineNickname.trim()"
              @click="createOfflineAccount"
            >
              {{ $t('common.add') }}
            </Button>
          </HoverCardContent>
        </HoverCard>
      </div>
    </div>

    <KitConfirmDialog
      :open="!!removeTarget"
      :title="$t('settings.accounts.remove_title')"
      :description="$t('settings.accounts.remove_text', { name: removeTarget?.name })"
      :confirm-label="$t('common.delete')"
      :loading="removing"
      @update:open="value => { if (!value) removeTarget = null }"
      @confirm="confirmRemove"
    />
  </KitPanel>
</template>
