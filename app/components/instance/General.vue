<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_TYPE_LABELS } from '~/types/instance'
import type { InstanceDir } from '~/types/backend'

const props = defineProps<{ instance: Instance }>()

const name = defineModel<string>('name', { required: true })
const icon = defineModel<string>('icon', { required: true })

const pickerOpen = ref(false)

const instanceStore = useInstanceStore()
const router = useRouter()
const toast = useAppToast()

const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const { total, last } = usePlaytime(() => props.instance)

const { t } = useI18n()

const facts = computed(() => [
  { label: t('instance.general.facts.id'), value: props.instance.id },
  { label: t('instance.general.facts.loader'), value: INSTANCE_TYPE_LABELS[props.instance.type] ?? props.instance.type },
  { label: 'Minecraft', value: props.instance.minecraftVersion },
  { label: t('instance.general.facts.loader_version'), value: props.instance.loaderVersion || '-' },
  { label: t('instance.general.facts.playtime'), value: formatPlaytime(total.value) || '-' },
  {
    label: t('instance.general.facts.last_played'),
    value: formatLastPlayed(props.instance.playtime?.lastPlayedAt ?? 0) || '-',
  },
  { label: t('instance.general.facts.last_session'), value: formatPlaytime(last.value) || '-' },
])

const status = computed(() => {
  if (running.value) return { text: t('instance.general.status.running'), tone: 'text-acid' }
  if (installing.value) return { text: t('instance.general.status.installing'), tone: 'text-fg-muted' }
  return props.instance.installed
    ? { text: t('instance.general.status.installed'), tone: 'text-fg-muted' }
    : { text: t('instance.general.status.absent'), tone: 'text-warning' }
})

const DIRS: { target: InstanceDir, labelKey: string, icon: string }[] = [
  { target: 'root', labelKey: 'instance.general.dir.root', icon: 'i-lucide-folder' },
  { target: 'minecraft', labelKey: 'instance.dir.minecraft', icon: 'i-lucide-folder-open' },
  { target: 'logs', labelKey: 'instance.general.dir.logs', icon: 'i-lucide-folder-clock' },
]

const openDir = (target: InstanceDir) => safeRun(
  () => call('open_instance_dir', { instanceId: props.instance.id, target }),
  { context: { instanceId: props.instance.id, action: t('instance.general.open_dir_action') } },
)

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
  <div class="space-y-6">
    <KitPanel
      index="01"
      :title="$t('instance.general.title')"
      icon="i-lucide-box"
    >
      <div class="space-y-7">
        <div class="flex items-center gap-5">
          <InstanceIcon
            :icon="icon"
            :type="instance.type"
            size="lg"
          />

          <div class="min-w-0 flex-1">
            <Label>{{ $t('instance.general.icon') }}</Label>
            <p
              class="mt-2 truncate font-mono text-caption text-fg-muted"
              :title="icon"
            >
              {{ icon || $t('instance.general.icon_default') }}
            </p>

            <div class="mt-3 flex items-center gap-3">
              <Button
                size="sm"
                icon="i-lucide-image"
                @click="pickerOpen = true"
              >
                {{ $t('instance.general.pick_icon') }}
              </Button>

              <Button
                v-if="icon"
                variant="quiet"
                icon="i-lucide-x"
                @click="icon = ''"
              >
                {{ $t('instance.general.clear_icon') }}
              </Button>
            </div>
          </div>
        </div>

        <KitField
          :label="$t('instance.general.name')"
          for="instance-general-name"
        >
          <Input
            id="instance-general-name"
            v-model="name"
            :placeholder="$t('instance.general.name_placeholder')"
          />
        </KitField>

        <KitDetails class="border-t border-line pt-6">
          <KitDetail
            v-for="fact in facts"
            :key="fact.label"
            :label="fact.label"
            :title="fact.value"
            mono
          >
            {{ fact.value }}
          </KitDetail>

          <KitDetail
            :label="$t('instance.general.status')"
            mono
          >
            <span :class="status.tone">{{ status.text }}</span>
          </KitDetail>
        </KitDetails>
      </div>
    </KitPanel>

    <KitPanel
      index="02"
      :title="$t('instance.general.files_title')"
      icon="i-lucide-hard-drive"
    >
      <div class="space-y-7">
        <div class="flex flex-wrap gap-3">
          <Button
            v-for="dir in DIRS"
            :key="dir.target"
            :icon="dir.icon"
            @click="openDir(dir.target)"
          >
            {{ $t(dir.labelKey) }}
          </Button>
        </div>

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
          :description="$t('instance.general.remove_hint')"
        >
          <Button
            variant="danger"
            icon="i-lucide-trash-2"
            :disabled="running"
            @click="removeOpen = true"
          >
            {{ $t('common.delete') }}
          </Button>
        </KitRow>

        <KitStatus
          v-if="running"
          dot="static"
        >
          {{ $t('instance.general.running_hint') }}
        </KitStatus>
      </div>
    </KitPanel>

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
        class="text-title leading-relaxed text-fg-muted"
      >
        <template #name>
          <span class="text-fg">{{ instance.name }}</span>
        </template>
      </i18n-t>
    </KitConfirmDialog>
  </div>
</template>
