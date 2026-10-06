<script setup lang="ts">
import type { Instance, InstanceSettings } from '~/types/instance'
import { INSTANCE_TYPE_LABELS } from '~/types/instance'

definePageMeta({
  layout: 'main',
})

type Tab = 'general' | 'mods' | 'castpack' | 'pack' | 'java' | 'logs'

const route = useRoute()
const toast = useAppToast()

const instanceStore = useInstanceStore()

const instanceId = computed(() => String(route.params.id ?? ''))
const instance = computed(() => instanceStore.getInstance(instanceId.value))

const { t } = useI18n()

const tab = ref<Tab>('general')
const saving = ref(false)
const exportOpen = ref(false)

const TABS = computed(() => {
  const items: { key: Tab, label: string, icon: string }[] = [
    { key: 'general', label: t('instance.tabs.general'), icon: 'i-lucide-box' },
    { key: 'mods', label: t('instance.tabs.mods'), icon: 'i-lucide-blocks' },
  ]

  if (instance.value?.castpack) {
    items.push({
      key: 'castpack',
      label: instance.value.castpack.origin === 'file' ? t('instance.tabs.cast_file') : 'CastPack',
      icon: 'i-lucide-layers',
    })
  }
  if (instance.value?.pack || instance.value?.localPack) {
    items.push({ key: 'pack', label: t('instance.tabs.pack'), icon: 'i-lucide-package' })
  }

  items.push({ key: 'java', label: 'Java', icon: 'i-lucide-cpu' })
  items.push({ key: 'logs', label: t('instance.tabs.logs'), icon: 'i-lucide-scroll-text' })

  return items.map((item, i) => ({ ...item, index: String(i + 1).padStart(2, '0') }))
})

watch(TABS, (items) => {
  if (!items.some(item => item.key === tab.value)) tab.value = 'general'
})

const running = computed(() => instanceStore.isRunning(instanceId.value))
const installing = computed(() => !!instanceStore.getInstall(instanceId.value))

const run = () => safeRun(
  () => instanceStore.playInstance(instanceId.value),
  { context: { instanceId: instanceId.value, action: t('instance.context.play') } },
)

const stop = () => safeRun(
  () => instanceStore.stopInstance(instanceId.value),
  { context: { instanceId: instanceId.value, action: t('instance.context.stop') } },
)

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
  <div class="min-h-full w-full px-8 pb-16 xl:px-14">
    <Tabs
      v-if="instance"
      v-model="tab"
      orientation="vertical"
      class="grid grid-cols-[15rem_minmax(0,1fr)] gap-14"
    >
      <aside class="sticky top-0 self-start pt-10">
        <Button
          to="/main"
          variant="quiet"
          class="group/back"
        >
          <Icon
            name="i-lucide-arrow-left"
            class="size-3.5 transition-transform duration-500 ease-deck group-hover/back:-translate-x-0.5"
          />
          {{ $t('instance.back') }}
        </Button>

        <div class="mt-2 space-y-4">
          <InstanceIcon
            :icon="draft.icon"
            :type="instance.type"
            size="lg"
          />

          <div class="min-w-0">
            <KitTitle
              size="sm"
              class="break-words"
              :title="instance.name"
            >
              {{ instance.name }}
            </KitTitle>

            <p class="mt-2 font-mono text-label uppercase tracking-caps text-fg-faint">
              {{ INSTANCE_TYPE_LABELS[instance.type] ?? instance.type }} · {{ instance.minecraftVersion }}
            </p>
          </div>
        </div>

        <TabsList class="mt-7">
          <TabsTrigger
            v-for="item in TABS"
            :key="item.key"
            :value="item.key"
            :icon="item.icon"
            :meta="item.index"
          >
            {{ item.label }}
          </TabsTrigger>
        </TabsList>

        <KitSaveBar
          class="mt-8"
          :saving="saving"
          :disabled="!canSave"
          :dirty="dirty"
          :dirty-label="$t('instance.unsaved')"
          @save="save"
          @reset="reset"
        >
          <Button
            v-if="running"
            icon="i-lucide-square"
            class="mt-3 w-full"
            @click="stop"
          >
            {{ $t('instance.stop_game') }}
          </Button>

          <Button
            v-else-if="instance.installed"
            icon="i-lucide-play"
            class="mt-3 w-full"
            :disabled="installing"
            @click="run"
          >
            {{ $t('instance.action.play') }}
          </Button>

          <Button
            variant="quiet"
            icon="i-lucide-file-down"
            class="mt-2"
            :disabled="installing"
            @click="exportOpen = true"
          >
            {{ $t('instance.export_cast') }}
          </Button>
        </KitSaveBar>

        <KitStatus
          v-if="!draft.name.trim()"
          tone="warning"
          class="mt-4"
        >
          {{ $t('instance.name_required') }}
        </KitStatus>
      </aside>

      <div class="min-w-0 pt-10">
        <InstanceGeneral
          v-show="tab === 'general'"
          v-model:name="draft.name"
          v-model:icon="draft.icon"
          :instance="instance"
          class="animate-rise"
        />

        <InstanceMods
          v-if="tab === 'mods'"
          :instance="instance"
          class="animate-rise"
        />

        <InstanceCastPack
          v-if="tab === 'castpack' && instance.castpack"
          :instance="instance"
          class="animate-rise"
        />

        <InstancePack
          v-if="tab === 'pack' && (instance.pack || instance.localPack)"
          :instance="instance"
          class="animate-rise"
        />

        <InstanceJava
          v-show="tab === 'java'"
          v-model="draft.settings"
          class="animate-rise"
        />

        <InstanceLogs
          v-if="tab === 'logs'"
          :instance-id="instance.id"
          class="animate-rise"
        />
      </div>
    </Tabs>

    <KitStatus
      v-else
      class="py-14"
    >
      {{ $t('instance.not_found') }}
    </KitStatus>

    <CastExportModal
      v-if="instance"
      v-model:open="exportOpen"
      :instance="instance"
    />

    <AppUnsavedChangesModal
      :guard="guard"
      :description="$t('instance.leave.description')"
      :blocked="$t('instance.name_required')"
      :discard-label="$t('unsaved.discard')"
    />
  </div>
</template>
