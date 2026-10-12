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
  <div class="space-y-6">
    <ul
      v-if="accountConfig?.accounts?.length"
      class="divide-y divide-line overflow-hidden rounded-md bg-panel ring-1 ring-card-line ring-inset"
    >
      <KitListItem
        v-for="(account, i) in accountConfig!.accounts"
        :key="`${account.type}-${account.name}-${i}`"
        :active="accountConfig?.selected === i"
        :active-label="$t('settings.accounts.active')"
        class="rounded-none px-4"
        @select="selectAccount(i)"
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

        <template #trailing>
          <Button
            variant="danger-ghost"
            size="icon-sm"
            icon="i-lucide-trash-2"
            :aria-label="$t('settings.accounts.remove')"
            :title="$t('settings.accounts.remove')"
            :disabled="!account.uuid"
            @click.stop="removeTarget = account"
          />
        </template>
      </KitListItem>
    </ul>

    <KitEmpty
      v-else
      icon="i-lucide-user-round"
    >
      {{ $t('settings.accounts.empty') }}
    </KitEmpty>

    <div class="flex flex-wrap items-end gap-6">
      <KitField label="Microsoft">
        <Button
          icon="simple-icons:microsoft"
          :loading="loggingIn"
          @click="createMicrosoftAccount"
        >
          {{ $t('layout.account.add') }}
        </Button>
      </KitField>

      <KitField
        :label="$t('settings.accounts.offline')"
        for="offline-nickname"
      >
        <div class="flex gap-2">
          <Input
            id="offline-nickname"
            v-model="offlineNickname"
            :placeholder="$t('settings.accounts.nickname')"
            class="w-56"
            @keydown.enter="createOfflineAccount"
          />
          <Button
            icon="i-lucide-plus"
            :disabled="!offlineNickname.trim()"
            @click="createOfflineAccount"
          >
            {{ $t('common.add') }}
          </Button>
        </div>
      </KitField>
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
  </div>
</template>
