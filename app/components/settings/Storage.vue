<script setup lang="ts">
import type { AppConfig } from '~/types/app'

const config = defineModel<AppConfig>({ required: true })

const { t } = useI18n()
const appStore = useAppStore()

async function pick() {
  const picked = await safeRun(() => call('pick_folder', {
    title: t('settings.launcher.dir.label'),
    directory: config.value.launcher.dir,
  }))

  if (picked) config.value.launcher.dir = picked
}

const open = () => safeRun(() => appStore.openLauncherDir())
</script>

<template>
  <KitField
    :label="$t('settings.launcher.dir.label')"
    for="settings-launcher-dir"
  >
    <div class="flex gap-2">
      <Input
        id="settings-launcher-dir"
        v-model="config.launcher.dir"
        placeholder="/path/to/launcher"
        font="mono"
      />
      <Button
        icon="i-lucide-folder-search"
        @click="pick"
      >
        {{ $t('common.pick') }}
      </Button>
      <Button
        size="icon"
        icon="i-lucide-folder-open"
        :aria-label="$t('settings.launcher.dir.open')"
        :title="$t('settings.launcher.dir.open')"
        @click="open"
      />
    </div>
  </KitField>
</template>
