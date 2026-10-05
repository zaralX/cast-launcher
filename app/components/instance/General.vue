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
const toast = useToast()

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
    : { text: t('instance.general.status.absent'), tone: 'text-amber-400' }
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
    <SettingsPanel
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
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ $t('instance.general.icon') }}
            </p>
            <p
              class="mt-2 truncate font-mono text-[11px] text-fg-muted"
              :title="icon"
            >
              {{ icon || $t('instance.general.icon_default') }}
            </p>

            <div class="mt-3 flex items-center gap-3">
              <AppButton
                class="h-8 px-3.5 text-[10px] tracking-[0.18em]"
                icon="i-lucide-image"
                @click="pickerOpen = true"
              >
                {{ $t('instance.general.pick_icon') }}
              </AppButton>

              <AppButton
                v-if="icon"
                tone="quiet"
                class="text-[10px] tracking-[0.18em]"
                icon="i-lucide-x"
                @click="icon = ''"
              >
                {{ $t('instance.general.clear_icon') }}
              </AppButton>
            </div>
          </div>
        </div>

        <SettingsField :label="$t('instance.general.name')">
          <UInput
            v-model="name"
            :placeholder="$t('instance.general.name_placeholder')"
            class="w-full"
          />
        </SettingsField>

        <dl class="grid gap-x-6 gap-y-4 border-t border-line pt-6 sm:grid-cols-2">
          <div
            v-for="fact in facts"
            :key="fact.label"
            class="min-w-0"
          >
            <dt class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ fact.label }}
            </dt>
            <dd
              class="mt-1.5 truncate font-mono text-[12px] text-fg-muted"
              :title="fact.value"
            >
              {{ fact.value }}
            </dd>
          </div>

          <div class="min-w-0">
            <dt class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ $t('instance.general.status') }}
            </dt>
            <dd
              class="mt-1.5 font-mono text-[12px]"
              :class="status.tone"
            >
              {{ status.text }}
            </dd>
          </div>
        </dl>
      </div>
    </SettingsPanel>

    <SettingsPanel
      index="02"
      :title="$t('instance.general.files_title')"
      icon="i-lucide-hard-drive"
    >
      <div class="space-y-7">
        <div class="flex flex-wrap gap-3">
          <AppButton
            v-for="dir in DIRS"
            :key="dir.target"
            class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
            :icon="dir.icon"
            @click="openDir(dir.target)"
          >
            {{ $t(dir.labelKey) }}
          </AppButton>
        </div>

        <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ instance.installed ? $t('instance.general.reinstall_title') : $t('instance.general.install_title') }}
            </p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              {{ $t('instance.general.reinstall_hint') }}
              <template v-if="instance.pack">
                {{ $t('instance.general.reinstall_pack_hint') }}
              </template>
              <template v-else-if="instance.localPack">
                {{ $t('instance.general.reinstall_local_hint') }}
              </template>
            </p>
          </div>

          <AppButton
            class="h-9 shrink-0 px-3.5 text-[10px] tracking-[0.18em]"
            :icon="instance.installed ? 'i-lucide-refresh-cw' : 'i-lucide-arrow-down-to-line'"
            :disabled="installing || running"
            @click="reinstall"
          >
            {{ installing ? $t('instance.general.installing') : instance.installed ? $t('instance.general.reinstall') : $t('instance.general.install') }}
          </AppButton>
        </div>

        <div class="flex items-center justify-between gap-6 border-t border-red-400/20 pt-6">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-red-400/80">
              {{ $t('instance.general.remove_section') }}
            </p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              {{ $t('instance.general.remove_hint') }}
            </p>
          </div>

          <UButton
            color="neutral"
            variant="ghost"
            class="h-9 shrink-0 justify-center border border-red-400/30 px-3.5 text-[10px] tracking-[0.18em] text-red-400 transition-colors duration-300 hover:bg-red-500 hover:text-white"
            icon="i-lucide-trash-2"
            :disabled="running"
            @click="() => { removeOpen = true }"
          >
            {{ $t('common.delete') }}
          </UButton>
        </div>

        <p
          v-if="running"
          class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint"
        >
          {{ $t('instance.general.running_hint') }}
        </p>
      </div>
    </SettingsPanel>

    <UModal
      v-model:open="pickerOpen"
      :title="$t('instance.general.icon_modal')"
      :ui="{ content: 'max-w-3xl' }"
    >
      <template #body>
        <InstanceIconPicker v-model="icon" />
      </template>
    </UModal>

    <UModal
      v-model:open="removeOpen"
      :title="$t('instance.general.remove_title')"
    >
      <template #body>
        <div class="space-y-6">
          <i18n-t
            keypath="instance.general.remove_text"
            tag="p"
            class="text-[13px] leading-relaxed text-fg-muted"
          >
            <template #name>
              <span class="text-fg">{{ instance.name }}</span>
            </template>
          </i18n-t>

          <div class="flex justify-end gap-3">
            <AppButton
              tone="quiet"
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              :disabled="removing"
              @click="removeOpen = false"
            >
              {{ $t('common.cancel') }}
            </AppButton>

            <UButton
              color="neutral"
              variant="ghost"
              class="h-9 justify-center border border-red-400/30 px-3.5 text-[10px] tracking-[0.18em] text-red-400 transition-colors duration-300 hover:bg-red-500 hover:text-white"
              icon="i-lucide-trash-2"
              :loading="removing"
              @click="remove"
            >
              {{ removing ? $t('instance.general.removing') : $t('instance.general.remove_forever') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
