<script setup lang="ts">
definePageMeta({
  layout: 'main',
})

type Section = 'look' | 'behavior' | 'accounts' | 'java' | 'storage' | 'import' | 'about'

interface Entry {
  value: Section
  labelKey: string
  icon: string
  keys: string[]
}

const GROUPS: { labelKey: string, items: Entry[] }[] = [
  {
    labelKey: 'settings.group.main',
    items: [
      {
        value: 'look',
        labelKey: 'settings.section.look',
        icon: 'i-lucide-palette',
        keys: [
          'settings.launcher.theme',
          'theme_editor.title',
          'theme.custom.import',
          'settings.launcher.system_light.label',
          'settings.launcher.system_dark.label',
          'settings.launcher.accent.label',
          'settings.launcher.language',
          'settings.launcher.compact.label',
        ],
      },
      {
        value: 'behavior',
        labelKey: 'settings.section.behavior',
        icon: 'i-lucide-app-window',
        keys: ['settings.launcher.after_launch.label', 'settings.launcher.auto_update.label', 'settings.launcher.telemetry.label'],
      },
      {
        value: 'accounts',
        labelKey: 'settings.accounts.title',
        icon: 'i-lucide-user-round',
        keys: ['settings.accounts.offline', 'settings.accounts.nickname', 'layout.account.add'],
      },
    ],
  },
  {
    labelKey: 'settings.group.game',
    items: [
      {
        value: 'java',
        labelKey: 'settings.java.title',
        icon: 'i-lucide-coffee',
        keys: ['settings.java.mode_label', 'settings.java.custom_path', 'settings.java.memory_title', 'settings.java.min_ram', 'settings.java.max_ram'],
      },
    ],
  },
  {
    labelKey: 'settings.group.system',
    items: [
      {
        value: 'storage',
        labelKey: 'settings.section.storage',
        icon: 'i-lucide-hard-drive',
        keys: ['settings.launcher.dir.label'],
      },
      {
        value: 'import',
        labelKey: 'settings.section.import',
        icon: 'i-lucide-import',
        keys: ['settings.import.title'],
      },
      {
        value: 'about',
        labelKey: 'settings.section.about',
        icon: 'i-lucide-info',
        keys: ['settings.about.check', 'layout.nav.credits'],
      },
    ],
  },
]

const ENTRIES = GROUPS.flatMap(group => group.items)

const route = useRoute()

const asked = route.query.section
const section = ref<Section>(ENTRIES.find(entry => entry.value === asked)?.value ?? 'look')

const query = ref('')

const store = useAppStore()
const { config } = storeToRefs(store)

const { t, locale } = useI18n()
const toast = useAppToast()

const groups = computed(() => {
  const needle = query.value.trim().toLocaleLowerCase(locale.value)
  if (!needle) return GROUPS

  const matches = (entry: Entry) => [entry.labelKey, ...entry.keys]
    .some(key => t(key).toLocaleLowerCase(locale.value).includes(needle))

  return GROUPS
    .map(group => ({ ...group, items: group.items.filter(matches) }))
    .filter(group => group.items.length)
})
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
  <Tabs
    v-model="section"
    orientation="vertical"
    class="flex min-h-full w-full"
  >
    <aside class="sticky top-0 flex h-[calc(100vh-2.5rem)] w-59 shrink-0 flex-col border-r border-line pt-6 pr-3 pb-4 pl-5">
      <KitSearchInput
        v-model="query"
        :placeholder="$t('settings.search')"
        :aria-label="$t('settings.search')"
      />

      <TabsList class="mt-1 min-h-0 flex-1 overflow-y-auto">
        <template
          v-for="group in groups"
          :key="group.labelKey"
        >
          <KitNavHeading>{{ $t(group.labelKey) }}</KitNavHeading>
          <TabsTrigger
            v-for="item in group.items"
            :key="item.value"
            :value="item.value"
            :icon="item.icon"
          >
            {{ $t(item.labelKey) }}
          </TabsTrigger>
        </template>

        <p
          v-if="!groups.length"
          class="px-2.5 py-3 text-caption text-fg-faint"
        >
          {{ $t('common.no_match') }}
        </p>
      </TabsList>
    </aside>

    <div class="min-w-0 flex-1 px-9 pt-6 pb-24">
      <template v-if="config">
        <div class="max-w-4xl">
          <TabsContent value="look">
            <SettingsLook v-model="config" />
          </TabsContent>
          <TabsContent value="behavior">
            <SettingsBehavior v-model="config" />
          </TabsContent>
          <TabsContent value="accounts">
            <SettingsAccounts />
          </TabsContent>
          <TabsContent value="java">
            <SettingsJava v-model="config" />
          </TabsContent>
          <TabsContent value="storage">
            <SettingsStorage v-model="config" />
          </TabsContent>
          <TabsContent value="import">
            <SettingsImport />
          </TabsContent>
          <TabsContent value="about">
            <SettingsAbout />
          </TabsContent>
        </div>
      </template>

      <KitEmpty
        v-else
        icon="i-lucide-settings"
      >
        {{ $t('settings.empty') }}
      </KitEmpty>
    </div>

    <KitSaveBar
      :saving="saving"
      :disabled="!config || !dirty"
      :dirty="dirty"
      :dirty-label="$t('settings.unsaved')"
      @save="saveConfig"
      @reset="reset"
    />

    <AppUnsavedChangesModal
      :guard="guard"
      :description="$t('settings.leave.description')"
      :discard-label="$t('settings.leave.discard')"
    />
  </Tabs>
</template>
