<script setup lang="ts">
import {storeToRefs} from "pinia";
import type {Account} from "~/types/account";
import {useAccountStore} from "~/stores/account";

const accountStore = useAccountStore();
const {accountConfig, loggingIn} = storeToRefs(accountStore)
const toast = useToast()

const offlineNickname = ref("")
const removeTarget = ref<Account | null>(null)
const removing = ref(false)

const createOfflineAccount = async () => {
  const name = offlineNickname.value.trim()
  if (!name) return

  await safeRun(() => accountStore.addOfflineAccount(name))
  offlineNickname.value = ""
}

const createMicrosoftAccount = () => safeRun(() => accountStore.microsoftLogin(), {code: "AUTH_FAILED"})

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
    title: `Аккаунт «${target.name}» удалён`,
    color: "success",
    icon: "i-lucide-trash-2"
  })
}
</script>

<template>
  <SettingsPanel
      index="02"
      title="Аккаунты"
      icon="i-lucide-user-round"
  >
    <div class="space-y-7">
      <ul v-if="accountConfig?.accounts?.length" class="border-t border-line">
        <li
            v-for="(account, i) in accountConfig!.accounts"
            :key="`${account.type}-${account.name}-${i}`"
            class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 px-4 transition-colors duration-300 hover:bg-ink-700"
            @click="selectAccount(i)"
        >
          <span
              class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
              :class="accountConfig?.selected === i ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
          />

          <img
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

          <UButton
              color="neutral"
              variant="ghost"
              icon="i-lucide-trash-2"
              aria-label="Удалить аккаунт"
              class="shrink-0 p-1 text-fg-faint transition-colors duration-300 hover:bg-transparent hover:text-red-400"
              :disabled="!account.uuid"
              @click.stop="removeTarget = account"
          />
        </li>
      </ul>

      <p v-else class="border-y border-line py-6 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
        Ни одного аккаунта
      </p>

      <div class="grid gap-4 sm:grid-cols-2">
        <AppButton
            block
            class="h-10 text-[10px] tracking-[0.18em]"
            icon="mdi:microsoft"
            :loading="loggingIn"
            @click="createMicrosoftAccount"
        >
          Microsoft
        </AppButton>

        <UPopover mode="hover">
          <UButton
              block
              color="neutral"
              variant="ghost"
              icon="i-lucide-globe"
              class="h-10 justify-center border border-line text-[10px] tracking-[0.18em] text-fg-muted hover:border-line-strong hover:bg-transparent hover:text-fg"
          >
            Оффлайн
          </UButton>

          <template #content>
            <div class="w-64 space-y-4 p-5">
              <SettingsField label="Никнейм">
                <UInput v-model="offlineNickname" placeholder="nickname" class="w-full"/>
              </SettingsField>
              <AppButton
                  block
                  class="h-9 text-[10px] tracking-[0.18em]"
                  :disabled="!offlineNickname.trim()"
                  @click="createOfflineAccount"
              >
                Добавить
              </AppButton>
            </div>
          </template>
        </UPopover>
      </div>
    </div>

    <UModal
        :open="!!removeTarget"
        title="Удаление аккаунта"
        @update:open="value => { if (!value) removeTarget = null }"
    >
      <template #body>
        <p class="text-[12px] leading-relaxed text-fg-muted">
          Аккаунт «{{ removeTarget?.name }}» будет убран из лаунчера. Сборки и их файлы это не затронет.
        </p>
      </template>

      <template #footer>
        <div class="flex w-full items-center justify-end gap-3">
          <AppButton tone="quiet" class="text-[10px] tracking-[0.18em]" @click="removeTarget = null">
            Отмена
          </AppButton>

          <AppButton
              class="h-8 text-[10px] tracking-[0.18em] hover:border-red-500 hover:before:bg-red-500 hover:text-white"
              icon="i-lucide-trash-2"
              :loading="removing"
              @click="confirmRemove"
          >
            Удалить
          </AppButton>
        </div>
      </template>
    </UModal>
  </SettingsPanel>
</template>
