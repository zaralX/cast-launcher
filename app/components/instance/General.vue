<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_DIR_KEYS, INSTANCE_DIRS, INSTANCE_TYPE_LABELS } from '~/types/instance'

const props = defineProps<{ instance: Instance }>()

const name = defineModel<string>('name', { required: true })
const icon = defineModel<string>('icon', { required: true })

const pickerOpen = ref(false)

const instanceStore = useInstanceStore()
const actions = useInstanceActions()
const router = useRouter()
const toast = useAppToast()

const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const { total, last } = usePlaytime(() => props.instance)

const { t } = useI18n()

const facts = computed(() => [
  { label: t('instance.general.facts.loader'), value: INSTANCE_TYPE_LABELS[props.instance.type] ?? props.instance.type },
  { label: t('instance.general.facts.loader_version'), value: props.instance.loaderVersion || '-' },
  { label: 'Minecraft', value: props.instance.minecraftVersion },
  { label: t('instance.general.facts.playtime'), value: formatPlaytime(total.value) || '-' },
  {
    label: t('instance.general.facts.last_played'),
    value: formatLastPlayed(props.instance.playtime?.lastPlayedAt ?? 0) || '-',
  },
  { label: t('instance.general.facts.last_session'), value: formatPlaytime(last.value) || '-' },
])

const status = computed(() => {
  if (running.value) return { text: t('instance.general.status.running'), tone: 'text-success' }
  if (installing.value) return { text: t('instance.general.status.installing'), tone: 'text-fg' }
  return props.instance.installed
    ? { text: t('instance.general.status.installed'), tone: 'text-fg' }
    : { text: t('instance.general.status.absent'), tone: 'text-warning' }
})

const DIR_ICONS: Record<typeof INSTANCE_DIRS[number], string> = {
  root: 'i-lucide-folder',
  minecraft: 'i-lucide-folder-open',
  logs: 'i-lucide-folder-clock',
}

const reinstall = () => safeRun(
  () => instanceStore.installInstance(props.instance.id),
  { context: { instanceId: props.instance.id, action: t('instance.general.reinstall_action') } },
)

const removeOpen = ref(false)
const removing = ref(false)

async function remove() {
  if (removing.value) return
  removing.value = true

  const result = await attempt(
    () => instanceStore.deleteInstance(props.instance.id),
    { context: { instanceId: props.instance.id, action: t('instance.remove_action') } },
  )

  removing.value = false

  if (!result.ok) return

  removeOpen.value = false
  toast.add({ title: t('instance.general.removed'), color: 'success', icon: 'i-lucide-trash-2' })
  await router.push('/main')
}
</script>

<template>
  <div class="grid grid-cols-2 items-start gap-3.5">
    <div class="flex flex-col gap-3.5">
      <KitPanel :title="$t('instance.general.identity_title')">
        <KitField
          :label="$t('instance.general.name')"
          :error="name.trim() ? undefined : $t('instance.name_required')"
          for="instance-general-name"
        >
          <Input
            id="instance-general-name"
            v-model="name"
            :aria-invalid="!name.trim() || undefined"
          />
        </KitField>

        <div class="mt-4 flex items-center gap-3">
          <InstanceIcon
            :icon="icon"
            :type="instance.type"
            :version="instance.minecraftVersion"
            size="md"
          />
          <p
            class="min-w-0 flex-1 truncate text-caption text-fg-faint"
            :title="icon"
          >
            {{ icon }}
          </p>
          <Button
            size="sm"
            icon="i-lucide-image"
            @click="pickerOpen = true"
          >
            {{ $t('instance.general.pick_icon') }}
          </Button>
          <Button
            v-if="icon"
            variant="ghost"
            size="sm"
            @click="icon = ''"
          >
            {{ $t('instance.general.clear_icon') }}
          </Button>
        </div>
      </KitPanel>

      <KitPanel :title="$t('instance.general.facts_title')">
        <dl class="-my-1.5">
          <KitFact
            v-for="fact in facts"
            :key="fact.label"
            :label="fact.label"
            :value="fact.value"
          />
          <KitFact :label="$t('instance.general.status')">
            <span :class="status.tone">{{ status.text }}</span>
          </KitFact>
          <KitFact :label="$t('instance.general.facts.id')">
            <span class="font-mono text-micro text-fg-muted">{{ instance.id }}</span>
          </KitFact>
        </dl>
      </KitPanel>
    </div>

    <div class="flex flex-col gap-3.5">
      <KitPanel :title="$t('instance.general.dirs_title')">
        <div class="flex flex-wrap gap-1.5">
          <Button
            v-for="target in INSTANCE_DIRS"
            :key="target"
            size="sm"
            :icon="DIR_ICONS[target]"
            @click="actions.openDir(instance.id, target)"
          >
            {{ $t(INSTANCE_DIR_KEYS[target]) }}
          </Button>
        </div>
      </KitPanel>

      <KitRows>
        <KitRow :label="instance.installed ? $t('instance.general.reinstall_title') : $t('instance.general.install_title')">
          <template #description>
            {{ $t('instance.general.reinstall_hint') }}
            <template v-if="instance.pack">
              {{ $t('instance.general.reinstall_pack_hint') }}
            </template>
            <template v-else-if="instance.localPack">
              {{ $t('instance.general.reinstall_local_hint') }}
            </template>
          </template>

          <Button
            size="sm"
            :icon="instance.installed ? 'i-lucide-refresh-cw' : 'i-lucide-arrow-down-to-line'"
            :disabled="installing || running"
            @click="reinstall"
          >
            {{ installing ? $t('instance.general.installing') : instance.installed ? $t('instance.general.reinstall') : $t('instance.general.install') }}
          </Button>
        </KitRow>

        <KitRow
          tone="danger"
          :label="$t('instance.general.remove_section')"
          :description="running ? $t('instance.general.running_hint') : $t('instance.general.remove_hint')"
        >
          <Button
            variant="danger"
            size="sm"
            icon="i-lucide-trash-2"
            :disabled="running"
            @click="removeOpen = true"
          >
            {{ $t('common.delete') }}
          </Button>
        </KitRow>
      </KitRows>
    </div>

    <Dialog v-model:open="pickerOpen">
      <DialogContent class="max-w-3xl">
        <DialogHeader>
          <DialogTitle>{{ $t('instance.general.icon_modal') }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <InstanceIconPicker v-model="icon" />
        </DialogBody>
      </DialogContent>
    </Dialog>

    <KitConfirmDialog
      v-model:open="removeOpen"
      :title="$t('instance.general.remove_title')"
      :confirm-label="removing ? $t('instance.general.removing') : $t('instance.general.remove_forever')"
      :loading="removing"
      @confirm="remove"
    >
      <i18n-t
        keypath="instance.general.remove_text"
        tag="p"
        class="text-body text-fg-muted"
      >
        <template #name>
          <span class="text-fg">{{ instance.name }}</span>
        </template>
      </i18n-t>
    </KitConfirmDialog>
  </div>
</template>
