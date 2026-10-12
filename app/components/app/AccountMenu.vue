<script setup lang="ts">
import { FOCUS_RING } from '@/lib/styles'
import { cn } from '@/lib/utils'

const accountStore = useAccountStore()
const { accounts, selected, accountConfig, loggingIn } = storeToRefs(accountStore)

const selectedIndex = computed({
  get: () => String(accountConfig.value?.selected ?? 0),
  set: (value: string) => {
    safeRun(() => accountStore.selectAccount(Number(value)))
  },
})

const menuOpen = ref(false)
const offlineOpen = ref(false)
const nickname = ref('')

const loginMicrosoft = () => safeRun(() => accountStore.microsoftLogin(), { code: 'AUTH_FAILED' })

async function addOffline() {
  const name = nickname.value.trim()
  if (!name) return

  await safeRun(() => accountStore.addOfflineAccount(name))
  nickname.value = ''
  offlineOpen.value = false
}
</script>

<template>
  <Tooltip :disabled="menuOpen">
    <TooltipTrigger as-child>
      <span class="mt-1.5 inline-flex">
        <DropdownMenu v-model:open="menuOpen">
          <DropdownMenuTrigger
            :aria-label="selected?.name ?? $t('settings.accounts.title')"
            :class="cn(FOCUS_RING, 'cursor-pointer rounded-sm transition-opacity duration-150 hover:opacity-85')"
          >
            <KitAvatar :name="selected?.name" />
          </DropdownMenuTrigger>

          <DropdownMenuContent
            side="right"
            align="end"
            :side-offset="10"
            class="w-80"
          >
            <template v-if="accounts.length">
              <DropdownMenuLabel>Minecraft</DropdownMenuLabel>
              <DropdownMenuRadioGroup v-model="selectedIndex">
                <DropdownMenuRadioItem
                  v-for="(account, i) in accounts"
                  :key="`${account.type}-${account.name}-${i}`"
                  :value="String(i)"
                  class="h-auto gap-3 py-2"
                >
                  <KitAvatar
                    :name="account.name"
                    :muted="i !== (accountConfig?.selected ?? 0)"
                  />
                  <span class="min-w-0 flex-1">
                    <span class="block truncate font-medium">{{ account.name }}</span>
                    <span class="block text-caption text-fg-faint">
                      {{ account.type === 'microsoft' ? 'Microsoft' : $t('settings.accounts.offline') }}
                    </span>
                  </span>
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </template>

            <DropdownMenuSub>
              <DropdownMenuSubTrigger
                icon="i-lucide-user-round-plus"
                class="h-auto py-2.5"
              >
                {{ $t('layout.account.add') }}
              </DropdownMenuSubTrigger>
              <DropdownMenuSubContent class="w-56">
                <DropdownMenuItem
                  icon="simple-icons:microsoft"
                  class="h-auto py-2"
                  :disabled="loggingIn"
                  @select="loginMicrosoft"
                >
                  <span class="min-w-0">
                    <span class="block font-medium">Microsoft</span>
                    <span class="block text-caption text-fg-faint">{{ $t('layout.account.microsoft_hint') }}</span>
                  </span>
                </DropdownMenuItem>
                <DropdownMenuItem
                  icon="i-lucide-globe"
                  class="h-auto py-2"
                  @select="offlineOpen = true"
                >
                  <span class="min-w-0">
                    <span class="block font-medium">{{ $t('settings.accounts.offline') }}</span>
                    <span class="block text-caption text-fg-faint">{{ $t('layout.account.offline_hint') }}</span>
                  </span>
                </DropdownMenuItem>
              </DropdownMenuSubContent>
            </DropdownMenuSub>

            <DropdownMenuSeparator />

            <DropdownMenuItem
              icon="i-lucide-settings"
              class="text-fg-muted"
              @select="navigateTo('/settings?section=accounts')"
            >
              {{ $t('layout.account.manage') }}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </span>
    </TooltipTrigger>
    <TooltipContent side="right">
      {{ selected?.name ?? $t('settings.accounts.title') }}
    </TooltipContent>
  </Tooltip>

  <Dialog v-model:open="offlineOpen">
    <DialogContent class="max-w-sm">
      <DialogHeader>
        <DialogTitle>{{ $t('layout.account.offline_title') }}</DialogTitle>
      </DialogHeader>
      <DialogBody>
        <KitField
          :label="$t('settings.accounts.nickname')"
          for="rail-offline-nickname"
        >
          <Input
            id="rail-offline-nickname"
            v-model="nickname"
            @keydown.enter="addOffline"
          />
        </KitField>
      </DialogBody>
      <DialogFooter>
        <Button
          variant="ghost"
          @click="offlineOpen = false"
        >
          {{ $t('common.cancel') }}
        </Button>
        <Button
          variant="primary"
          icon="i-lucide-plus"
          :disabled="!nickname.trim()"
          @click="addOffline"
        >
          {{ $t('common.add') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
