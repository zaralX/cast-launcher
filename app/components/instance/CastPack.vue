<script setup lang="ts">
import type { Instance } from '~/types/instance'
import type { CastPackUpdate } from '~/types/castpack'

const props = defineProps<{ instance: Instance }>()

const instanceStore = useInstanceStore()
const castpackStore = useCastPackStore()
const toast = useAppToast()

const checking = ref(false)
const update = ref<CastPackUpdate | null>(null)
const updateFromFile = ref(false)

const source = computed(() => props.instance.castpack)
const fromFile = computed(() => source.value?.origin === 'file')
const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const autoupdate = computed({
  get: () => source.value?.autoupdate ?? false,
  set: (enabled: boolean) => toggle(enabled),
})

const { t } = useI18n()

const blocked = computed(() => {
  if (running.value) return t('instance.pack.blocked_running')
  if (installing.value) return t('instance.pack.blocked_installing')
  return null
})

const facts = computed(() => {
  const version = {
    label: t('instance.castpack.facts.version'),
    value: source.value?.version || t('instance.castpack.facts.version_absent'),
  }

  if (fromFile.value) {
    return [
      { label: t('instance.castpack.facts.pack_id'), value: source.value?.catalogId ?? '-' },
      version,
      { label: t('instance.castpack.facts.source'), value: t('instance.castpack.facts.source_file') },
    ]
  }

  return [
    { label: t('instance.castpack.facts.pack'), value: source.value?.catalogId ?? '-' },
    version,
    {
      label: t('instance.castpack.facts.autoupdate'),
      value: source.value?.autoupdate
        ? t('instance.castpack.facts.autoupdate_on')
        : t('instance.castpack.facts.autoupdate_off'),
    },
    { label: t('instance.castpack.facts.manifest'), value: source.value?.manifestUrl ?? '-' },
  ]
})

const announceImport = useImportToast()

function onUpdated(instanceId: string, updated: boolean) {
  updateFromFile.value = false
  announceImport(instanceId, updated)
}

async function check() {
  if (checking.value || fromFile.value) return

  checking.value = true

  const result = await attempt(() => castpackStore.checkUpdate(props.instance.id), {
    code: 'NETWORK',
    context: { instanceId: props.instance.id, action: t('instance.castpack.check_action') },
  })

  checking.value = false

  if (result.ok) update.value = result.value
}

onMounted(check)

async function toggle(enabled: boolean) {
  const result = await attempt(() => castpackStore.setAutoupdate(props.instance.id, enabled), {
    context: { instanceId: props.instance.id, action: t('instance.castpack.toggle_action') },
  })

  if (!result.ok) return

  toast.add({
    title: enabled ? t('instance.castpack.autoupdate_on_title') : t('instance.castpack.autoupdate_off_title'),
    description: enabled
      ? t('instance.castpack.autoupdate_on_hint')
      : t('instance.castpack.autoupdate_off_hint'),
    color: 'success',
    icon: 'i-lucide-refresh-cw',
  })
}

async function reinstall() {
  if (blocked.value) return

  const started = await attempt(() => instanceStore.installInstance(props.instance.id), {
    code: 'NETWORK',
    context: { instanceId: props.instance.id, action: t('instance.castpack.reinstall_action') },
  })

  if (!started.ok) return

  toast.add({
    title: t('instance.castpack.repair_title'),
    description: t('instance.castpack.repair_hint'),
    color: 'success',
    icon: 'i-lucide-refresh-cw',
  })
}

const openSite = (url: string) => safeRun(() => call('open_url', { url }))
</script>

<template>
  <div class="space-y-6">
    <KitPanel
      index="01"
      :title="$t('instance.castpack.title')"
      icon="i-lucide-layers"
    >
      <p
        v-if="!source"
        class="text-body leading-relaxed text-fg-muted"
      >
        {{ $t('instance.castpack.not_castpack') }}
      </p>

      <div
        v-else-if="fromFile"
        class="space-y-7"
      >
        <KitNote
          icon="i-lucide-file-archive"
          tone="accent"
        >
          {{ $t('instance.castpack.file_hint') }}
        </KitNote>

        <KitRow :description="$t('instance.castpack.files_hint')">
          <Button
            variant="quiet"
            icon="i-lucide-wrench"
            :disabled="!!blocked"
            @click="reinstall"
          >
            {{ $t('instance.castpack.repair') }}
          </Button>

          <Button
            icon="i-lucide-file-up"
            :disabled="!!blocked"
            @click="updateFromFile = true"
          >
            {{ $t('instance.castpack.update_from_file') }}
          </Button>
        </KitRow>

        <KitStatus
          v-if="blocked"
          dot="static"
        >
          {{ blocked }}
        </KitStatus>
      </div>

      <div
        v-else
        class="space-y-7"
      >
        <Alert
          v-if="update?.available"
          variant="accent"
        >
          <i18n-t
            keypath="instance.castpack.update_available"
            tag="span"
          >
            <template #version>
              <span class="text-fg">{{ update.version }}</span>
            </template>
          </i18n-t>
          {{ $t('instance.castpack.update_auto') }}

          <p
            v-if="update.changelog"
            class="mt-2 whitespace-pre-line"
          >
            {{ update.changelog }}
          </p>

          <template #action>
            <Button
              variant="quiet"
              icon="i-lucide-arrow-down-to-line"
              :disabled="!!blocked"
              @click="reinstall"
            >
              {{ $t('instance.castpack.update') }}
            </Button>
          </template>
        </Alert>

        <KitNote
          v-else-if="update?.error"
          icon="i-lucide-wifi-off"
        >
          {{ $t('instance.castpack.check_failed', { error: uiText(update.error.text) }) }}
        </KitNote>

        <KitNote
          v-else-if="update"
          icon="i-lucide-check"
          tone="accent"
        >
          {{ $t('instance.castpack.up_to_date') }}
        </KitNote>

        <KitField
          :label="$t('instance.castpack.autoupdate')"
          :hint="$t('instance.castpack.autoupdate_hint')"
        >
          <Switch v-model="autoupdate" />
        </KitField>

        <KitRow :description="$t('instance.castpack.files_hint')">
          <Button
            variant="quiet"
            icon="i-lucide-rotate-cw"
            :loading="checking"
            @click="check"
          >
            {{ $t('instance.castpack.check') }}
          </Button>

          <Button
            icon="i-lucide-wrench"
            :disabled="!!blocked"
            @click="reinstall"
          >
            {{ $t('instance.castpack.repair') }}
          </Button>
        </KitRow>

        <KitStatus
          v-if="blocked"
          dot="static"
        >
          {{ blocked }}
        </KitStatus>
      </div>
    </KitPanel>

    <KitPanel
      v-if="source?.changelog"
      index="02"
      :title="$t('instance.castpack.changelog_title')"
      icon="i-lucide-scroll-text"
    >
      <p class="text-body leading-relaxed whitespace-pre-line text-fg-muted">
        {{ source.changelog }}
      </p>
    </KitPanel>

    <KitPanel
      :index="source?.changelog ? '03' : '02'"
      :title="$t('instance.castpack.source_title')"
      icon="i-lucide-link"
    >
      <KitDetails>
        <KitDetail
          v-for="fact in facts"
          :key="fact.label"
          :label="fact.label"
          :title="fact.value"
          mono
        >
          {{ fact.value }}
        </KitDetail>
      </KitDetails>

      <Button
        v-if="source?.manifestUrl && !fromFile"
        variant="quiet"
        icon="i-lucide-external-link"
        class="mt-3"
        @click="openSite(source.manifestUrl)"
      >
        {{ $t('instance.castpack.open_manifest') }}
      </Button>
    </KitPanel>

    <Dialog v-model:open="updateFromFile">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ $t('instance.castpack.update_from_file_title', { name: instance.name }) }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <ImportPackModalBody
            :update-target="instance.id"
            @imported="onUpdated"
          />
        </DialogBody>
      </DialogContent>
    </Dialog>
  </div>
</template>
