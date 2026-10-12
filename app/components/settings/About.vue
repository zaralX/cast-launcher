<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'

const { t } = useI18n()
const appStore = useAppStore()
const toast = useAppToast()

const version = ref('')
const checking = ref(false)

onMounted(async () => {
  try {
    version.value = await getVersion()
  }
  catch {
    version.value = ''
  }
})

async function check() {
  if (checking.value) return
  checking.value = true

  const result = await attempt(() => appStore.updateApp(), { context: { action: t('settings.about.update_action') } })

  checking.value = false

  if (result.ok && !result.value) {
    toast.add({ title: t('settings.about.latest'), color: 'success', icon: 'i-lucide-check' })
  }
}
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

    <div class="flex gap-2">
      <Button
        icon="i-lucide-refresh-cw"
        :loading="checking"
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
