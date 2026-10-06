<script setup lang="ts">
definePageMeta({
  layout: 'main',
})

const store = useAppStore()
const { config } = storeToRefs(store)

const { t } = useI18n()
const toast = useAppToast()
const saving = ref(false)

const saved = ref<string | null>(null)

watch(config, (value) => {
  if (value && saved.value === null) saved.value = JSON.stringify(value)
}, { immediate: true })

const dirty = computed(() => {
  if (!config.value || saved.value === null) return false
  return JSON.stringify(config.value) !== saved.value
})

async function saveConfig() {
  if (!config.value || saving.value) return false
  saving.value = true
  try {
    await store.updateConfig(config.value)
    saved.value = JSON.stringify(config.value)
    toast.add({
      title: t('settings.saved'),
      color: 'success',
      icon: 'i-lucide-save',
    })
    return true
  }
  catch {
    toast.add({
      title: t('settings.save_failed.title'),
      description: t('settings.save_failed.hint'),
      color: 'error',
      icon: 'i-lucide-save',
    })
    return false
  }
  finally {
    saving.value = false
  }
}

function reset() {
  if (saved.value !== null) config.value = JSON.parse(saved.value)
}

const guard = useUnsavedChanges({
  dirty,
  save: saveConfig,
  discard: reset,
})
</script>

<template>
  <div class="min-h-full w-full px-8 pb-8 xl:px-14">
    <div class="grid grid-cols-[15rem_minmax(0,1fr)] gap-14">
      <aside class="sticky top-0 self-start pt-10">
        <KitPageHeader
          :eyebrow="$t('settings.eyebrow')"
          :title="$t('settings.title')"
          :description="$t('settings.subtitle')"
        />

        <KitSaveBar
          class="mt-8"
          :saving="saving"
          :disabled="!config || !dirty"
          :dirty="dirty"
          :dirty-label="$t('settings.unsaved')"
          @save="saveConfig"
          @reset="reset"
        />
      </aside>

      <div
        v-if="config"
        class="space-y-6 pt-10"
      >
        <SettingsLauncher
          v-model="config"
          class="animate-rise"
        />
        <SettingsAccounts class="animate-rise [animation-delay:80ms]" />
        <SettingsJava
          v-model="config"
          class="animate-rise [animation-delay:160ms]"
        />
        <SettingsImport class="animate-rise [animation-delay:240ms]" />
      </div>

      <KitStatus
        v-else
        class="py-14"
      >
        {{ $t('settings.empty') }}
      </KitStatus>
    </div>

    <AppUnsavedChangesModal
      :guard="guard"
      :description="$t('settings.leave.description')"
      :discard-label="$t('settings.leave.discard')"
    />
  </div>
</template>
