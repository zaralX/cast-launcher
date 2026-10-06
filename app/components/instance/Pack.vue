<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_TYPE_LABELS, LOCAL_PACK_KIND_LABELS, PACK_PROVIDER_LABELS } from '~/types/instance'
import type { BlockedFile, PackVersion } from '~/types/catalog'
import type { LauncherError } from '~/utils/error'

const props = defineProps<{ instance: Instance }>()

const instanceStore = useInstanceStore()
const toast = useAppToast()

const loading = ref(true)
const loadError = ref<LauncherError | null>(null)
const updating = ref(false)

const versions = ref<PackVersion[]>([])
const versionId = ref('')

const blockedFiles = ref<BlockedFile[]>([])

const pack = computed(() => props.instance.pack)
const local = computed(() => props.instance.localPack)
const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const selected = computed(() => versions.value.find(version => version.id === versionId.value) ?? null)

const versionItems = computed(() => versions.value.map(version => ({
  label: versionLabel(version),
  value: version.id,
  disabled: !version.supported,
})))

const latest = computed(() => versions.value.find(version => version.supported) ?? null)

const updateAvailable = computed(() =>
  !!latest.value && !!pack.value && latest.value.id !== pack.value.versionId,
)

const { t } = useI18n()

const changed = computed(() => !!pack.value && versionId.value !== pack.value.versionId)

const blocked = computed(() => {
  if (running.value) return t('instance.pack.blocked_running')
  if (installing.value) return t('instance.pack.blocked_installing')
  return null
})

const canApply = computed(() =>
  !loading.value && !loadError.value && !updating.value && !blocked.value && !!selected.value?.supported,
)

const facts = computed(() => {
  const file = local.value

  if (file) {
    return [
      { label: t('instance.pack.facts.source'), value: t('instance.pack.facts.file') },
      { label: t('instance.pack.facts.format'), value: LOCAL_PACK_KIND_LABELS[file.kind] },
      { label: t('instance.pack.facts.name'), value: file.name || '-' },
      { label: t('instance.pack.facts.version'), value: file.version || '-' },
    ]
  }

  return [
    { label: t('instance.pack.facts.source'), value: pack.value ? PACK_PROVIDER_LABELS[pack.value.provider] : '-' },
    { label: t('instance.pack.facts.project'), value: pack.value?.projectId ?? '-' },
    { label: t('instance.pack.facts.current_version'), value: pack.value?.versionNumber || pack.value?.versionId || '-' },
    { label: t('instance.pack.facts.archive'), value: pack.value?.fileName || '-' },
  ]
})

async function loadVersions() {
  if (!pack.value) return

  loading.value = true
  loadError.value = null

  try {
    versions.value = await call('list_pack_versions', {
      provider: pack.value.provider,
      projectId: pack.value.projectId,
    })
  }
  catch (e) {
    loadError.value = captureError(e, {
      code: 'NETWORK',
      context: { instanceId: props.instance.id, action: t('instance.pack.versions_action') },
    })
  }
  finally {
    loading.value = false
  }
}

async function loadBlocked() {
  blockedFiles.value = await safeRun(() => call('list_pack_blocked', { instanceId: props.instance.id })) ?? []
}

onMounted(() => {
  loadVersions()
  loadBlocked()
})

watch(installing, (running) => {
  if (!running) loadBlocked()
})

watch(() => pack.value?.versionId, (id) => {
  versionId.value = id ?? ''
}, { immediate: true })

const selectLatest = () => {
  if (latest.value) versionId.value = latest.value.id
}

const openPage = (url: string) => safeRun(() => call('open_url', { url }))

const openMods = () => safeRun(() => call('open_instance_dir', {
  instanceId: props.instance.id,
  target: 'minecraft',
}))

async function apply() {
  if (!canApply.value || !changed.value) return

  updating.value = true

  const context = { instanceId: props.instance.id, action: t('instance.pack.switch_action') }

  const switched = await attempt(
    () => call('set_instance_pack_version', { instanceId: props.instance.id, versionId: versionId.value }),
    { context },
  )

  if (!switched.ok) {
    updating.value = false
    return
  }

  await safeRun(() => instanceStore.installInstance(props.instance.id), { code: 'NETWORK', context })

  updating.value = false

  toast.add({
    title: t('instance.pack.switched', { version: switched.value.pack?.versionNumber ?? t('instance.pack.switched_fallback') }),
    description: t('instance.pack.switched_hint'),
    color: 'success',
    icon: 'i-lucide-refresh-cw',
  })
}
</script>

<template>
  <div class="space-y-6">
    <KitPanel
      index="01"
      :title="local ? $t('instance.pack.title_local') : $t('instance.pack.title_remote')"
      icon="i-lucide-package"
    >
      <KitStatGrid v-if="local">
        <KitStat
          label="Minecraft"
          :value="instance.minecraftVersion"
        />
        <KitStat
          :label="$t('instance.pack.loader')"
          :value="INSTANCE_TYPE_LABELS[instance.type]"
        />
      </KitStatGrid>

      <p
        v-else-if="!pack"
        class="text-body leading-relaxed text-fg-muted"
      >
        {{ $t('instance.pack.manual_instance') }}
      </p>

      <KitLoading
        v-else-if="loading"
        :label="$t('instance.pack.loading')"
        class="py-10"
      />

      <KitLoadError
        v-else-if="loadError"
        :error="loadError"
        @retry="loadVersions"
      />

      <div
        v-else
        class="space-y-7"
      >
        <Alert
          v-if="updateAvailable"
          variant="accent"
          class="items-center"
        >
          {{ $t('instance.pack.update_available') }}
          <span class="text-fg">{{ latest?.versionNumber || latest?.name }}</span>

          <template #action>
            <Button
              variant="quiet"
              @click="selectLatest"
            >
              {{ $t('instance.pack.select_latest') }}
            </Button>
          </template>
        </Alert>

        <KitField :label="$t('instance.pack.version')">
          <KitSelectMenu
            v-model="versionId"
            :items="versionItems"
            :search-placeholder="$t('instance.pack.version_search')"
          />
        </KitField>

        <KitStatGrid>
          <KitStat
            label="Minecraft"
            :value="selected?.minecraftVersion ?? instance.minecraftVersion"
          />
          <KitStat
            :label="$t('instance.pack.loader')"
            :value="selected?.loader ? INSTANCE_TYPE_LABELS[selected.loader] : INSTANCE_TYPE_LABELS[instance.type]"
          />
        </KitStatGrid>

        <KitNote
          v-if="selected && !selected.supported"
          icon="i-lucide-triangle-alert"
        >
          {{ $t('instance.pack.unsupported', { reason: unsupportedReason(selected) }) }}
        </KitNote>

        <KitRow :description="$t('instance.pack.apply_hint')">
          <Button
            icon="i-lucide-refresh-cw"
            :loading="updating"
            :disabled="!canApply || !changed"
            @click="apply"
          >
            {{ updating ? $t('instance.pack.updating') : $t('instance.pack.update') }}
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
      v-if="blockedFiles.length"
      index="02"
      :title="$t('instance.pack.manual_title')"
      icon="i-lucide-hand"
    >
      <p class="text-body leading-relaxed text-fg-muted">
        {{ $t('instance.pack.manual_hint') }}
      </p>

      <ul class="mt-5 divide-y divide-line border border-line">
        <li
          v-for="file in blockedFiles"
          :key="file.targetPath"
          class="flex items-center gap-4 px-4 py-3"
        >
          <div class="min-w-0 flex-1">
            <p
              class="truncate text-body text-fg"
              :title="file.fileName"
            >
              {{ file.fileName }}
            </p>
            <p
              class="mt-1 truncate font-mono text-label text-fg-faint"
              :title="file.targetPath"
            >
              {{ file.targetPath }}
            </p>
          </div>

          <Button
            v-if="file.websiteUrl"
            variant="quiet"
            icon="i-lucide-external-link"
            @click="openPage(file.websiteUrl)"
          >
            {{ $t('instance.pack.open') }}
          </Button>
        </li>
      </ul>

      <Button
        variant="quiet"
        icon="i-lucide-folder-open"
        class="mt-3"
        @click="openMods"
      >
        {{ $t('instance.pack.open_game_folder') }}
      </Button>
    </KitPanel>

    <KitPanel
      :index="blockedFiles.length ? '03' : '02'"
      :title="$t('instance.pack.content_title')"
      icon="i-lucide-list"
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
    </KitPanel>
  </div>
</template>
