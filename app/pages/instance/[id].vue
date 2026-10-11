<script setup lang="ts">
import type { Instance, InstanceSettings } from '~/types/instance'
import type { TabItem } from '~/types/ui'

definePageMeta({
  layout: 'main',
})

type Section = 'mods' | 'settings' | 'logs'
type SettingsSection = 'general' | 'castpack' | 'pack' | 'launch'

const SECTIONS: Section[] = ['mods', 'settings', 'logs']

const route = useRoute()
const toast = useAppToast()

const instanceStore = useInstanceStore()
const modsStore = useModsStore()

const instanceId = computed(() => String(route.params.id ?? ''))
const instance = computed(() => instanceStore.getInstance(instanceId.value))

const { t } = useI18n()

const hasMods = computed(() => instance.value?.type !== 'vanilla')

function initialSection(): Section {
  const asked = route.query.tab
  if (typeof asked === 'string' && (SECTIONS as string[]).includes(asked)) return asked as Section
  return hasMods.value ? 'mods' : 'settings'
}

const section = ref<Section>(initialSection())
const settingsSection = ref<SettingsSection>('general')
const saving = ref(false)
const exportOpen = ref(false)

watch(instanceId, () => {
  section.value = initialSection()
  settingsSection.value = 'general'
})

watch(hasMods, (value) => {
  if (!value && section.value === 'mods') section.value = 'settings'
})

const modsCount = computed(() => (modsStore.isLoaded(instanceId.value)
  ? modsStore.listOf(instanceId.value).length
  : undefined))

const mainTabs = computed<TabItem<Section>[]>(() => hasMods.value
  ? [{ value: 'mods', label: t('instance.tabs.mods'), meta: modsCount.value }]
  : [])

const toolTabs = computed<TabItem<Section>[]>(() => [
  { value: 'settings', label: t('instance.tabs.settings'), icon: 'i-lucide-settings' },
  { value: 'logs', label: t('instance.tabs.logs'), icon: 'i-lucide-scroll-text' },
])

const settingsTabs = computed(() => {
  const items: TabItem<SettingsSection>[] = [
    { value: 'general', label: t('instance.tabs.general'), icon: 'i-lucide-sliders-horizontal' },
  ]

  if (instance.value?.castpack) {
    items.push({
      value: 'castpack',
      label: instance.value.castpack.origin === 'file' ? t('instance.tabs.cast_file') : 'CastPack',
      icon: 'i-lucide-layers',
    })
  }
  if (instance.value?.pack || instance.value?.localPack) {
    items.push({ value: 'pack', label: t('instance.tabs.pack'), icon: 'i-lucide-package' })
  }

  items.push({ value: 'launch', label: t('instance.tabs.launch'), icon: 'i-lucide-rocket' })

  return items
})

watch(settingsTabs, (items) => {
  if (!items.some(item => item.value === settingsSection.value)) settingsSection.value = 'general'
})

const draft = ref({
  name: '',
  description: '',
  icon: '',
  settings: emptyInstanceSettings(),
})

function snapshot(source: Instance) {
  return {
    name: source.name,
    description: source.description ?? '',
    icon: source.icon ?? '',
    settings: { ...emptyInstanceSettings(), ...(source.settings ?? {}) } as InstanceSettings,
  }
}

watch(instanceId, () => {
  if (instance.value) draft.value = snapshot(instance.value)
}, { immediate: true })

watch(instance, (loaded, previous) => {
  if (loaded && !previous) draft.value = snapshot(loaded)
})

const dirty = computed(() => {
  if (!instance.value) return false
  return JSON.stringify(draft.value) !== JSON.stringify(snapshot(instance.value))
})

const canSave = computed(() => !!instance.value && dirty.value && !!draft.value.name.trim() && !saving.value)

async function save() {
  if (!canSave.value || !instance.value) return false

  saving.value = true

  const result = await attempt(() => instanceStore.updateInstance(instanceId.value, {
    name: draft.value.name.trim(),
    description: draft.value.description.trim(),
    icon: draft.value.icon,
    settings: draft.value.settings,
  }), { context: { instanceId: instanceId.value, action: t('instance.save_action') } })

  saving.value = false

  if (!result.ok) {
    toast.add({
      title: t('instance.save_failed.title'),
      description: t('instance.save_failed.hint'),
      color: 'error',
      icon: 'i-lucide-save',
    })
    return false
  }

  draft.value = snapshot(result.value)

  toast.add({ title: t('instance.saved'), color: 'success', icon: 'i-lucide-save' })

  return true
}

function reset() {
  if (instance.value) draft.value = snapshot(instance.value)
}

const guard = useUnsavedChanges({
  dirty,
  canSave: () => !!draft.value.name.trim(),
  save,
  discard: reset,
})
</script>

<template>
  <div class="flex min-h-full w-full flex-col">
    <Tabs
      v-if="instance"
      v-model="section"
      class="flex flex-1 flex-col"
    >
      <InstanceHeader
        :instance="instance"
        :icon="draft.icon"
        :sections="mainTabs"
        :tools="toolTabs"
        @export="exportOpen = true"
      />

      <div class="flex-1 px-8 pt-4 pb-24">
        <TabsContent
          v-if="hasMods"
          value="mods"
        >
          <InstanceMods :instance="instance" />
        </TabsContent>

        <TabsContent value="settings">
          <Tabs
            v-model="settingsSection"
            orientation="vertical"
            class="flex items-start gap-7"
          >
            <div class="sticky top-15 w-47 shrink-0">
              <TabsList>
                <TabsTrigger
                  v-for="item in settingsTabs"
                  :key="item.value"
                  :value="item.value"
                  :icon="item.icon"
                >
                  {{ item.label }}
                </TabsTrigger>
              </TabsList>

              <p
                v-if="settingsSection === 'launch'"
                class="mt-3.5 px-2.5 text-caption text-fg-faint"
              >
                {{ $t('instance.java.inherit_hint') }}
              </p>
            </div>

            <div class="min-w-0 flex-1">
              <TabsContent value="general">
                <InstanceGeneral
                  v-model:name="draft.name"
                  v-model:icon="draft.icon"
                  :instance="instance"
                />
              </TabsContent>

              <TabsContent
                v-if="instance.castpack"
                value="castpack"
              >
                <InstanceCastPack :instance="instance" />
              </TabsContent>

              <TabsContent
                v-if="instance.pack || instance.localPack"
                value="pack"
              >
                <InstancePack :instance="instance" />
              </TabsContent>

              <TabsContent value="launch">
                <InstanceJava v-model="draft.settings" />
              </TabsContent>
            </div>
          </Tabs>
        </TabsContent>

        <TabsContent value="logs">
          <InstanceLogs :instance-id="instance.id" />
        </TabsContent>
      </div>
    </Tabs>

    <KitEmpty
      v-else
      icon="i-lucide-box"
      class="m-8"
    >
      {{ $t('instance.not_found') }}
    </KitEmpty>

    <CastExportModal
      v-if="instance"
      v-model:open="exportOpen"
      :instance="instance"
    />

    <KitSaveBar
      :saving="saving"
      :disabled="!canSave"
      :dirty="dirty"
      :dirty-label="$t('instance.unsaved')"
      @save="save"
      @reset="reset"
    />

    <AppUnsavedChangesModal
      :guard="guard"
      :description="$t('instance.leave.description')"
      :blocked="$t('instance.name_required')"
      :discard-label="$t('unsaved.discard')"
    />
  </div>
</template>
