<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'

const { t } = useI18n()
const appStore = useAppStore()
const { update, updateBusy } = storeToRefs(appStore)
const toast = useAppToast()

const version = ref('')

onMounted(async () => {
  try {
    version.value = await getVersion()
  }
  catch {
    version.value = ''
  }
})

async function check() {
  const result = await attempt(() => appStore.checkUpdate(), { context: { action: t('settings.about.update_action') } })

  if (result.ok && !result.value) {
    toast.add({ title: t('settings.about.latest'), color: 'success', icon: 'i-lucide-check' })
  }
}

const install = () => safeRun(() => appStore.installUpdate(), { context: { action: t('settings.about.update_action') } })
</script>

<template>
  <div class="space-y-5">
    <div class="flex items-center gap-4">
      <img
        src="/logo.svg"
        alt=""
        class="size-14"
      >
      <div>
        <p class="font-unbounded text-heading font-semibold tracking-heading text-fg classic:uppercase">
          Cast Launcher
        </p>
        <p
          v-if="version"
          class="mt-0.5 text-caption text-fg-faint"
        >
          {{ $t('settings.about.version', { version }) }}
        </p>
      </div>
    </div>

    <KitStatus
      v-if="update"
      tone="accent"
    >
      {{ $t('settings.about.available', { version: update.version }) }}
    </KitStatus>

    <div class="flex gap-2">
      <Button
        v-if="update"
        variant="primary"
        :icon="update.downloaded ? 'i-lucide-rotate-cw' : 'i-lucide-circle-arrow-up'"
        :loading="updateBusy === 'installing' || updateBusy === 'downloading'"
        @click="install"
      >
        {{ update.downloaded ? $t('update.restart') : $t('update.install') }}
      </Button>
      <Button
        v-else
        icon="i-lucide-refresh-cw"
        :loading="updateBusy === 'checking'"
        @click="check"
      >
        {{ $t('settings.about.check') }}
      </Button>
      <Button
        variant="ghost"
        icon="i-lucide-info"
        to="/credits"
      >
        {{ $t('layout.nav.credits') }}
      </Button>
    </div>
  </div>
</template>
