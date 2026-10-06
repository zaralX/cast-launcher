<script setup lang="ts">
import { v4 } from 'uuid'
import type { PackHit, PackVersion } from '~/types/catalog'
import { INSTANCE_TYPE_LABELS, PACK_PROVIDER_LABELS } from '~/types/instance'
import type { LauncherError } from '~/utils/error'

const props = defineProps<{ hit: PackHit }>()

const emit = defineEmits<{ installed: [instanceId: string] }>()

const instanceStore = useInstanceStore()

const loading = ref(true)
const loadError = ref<LauncherError | null>(null)
const creating = ref(false)

const versions = ref<PackVersion[]>([])
const versionId = ref('')

const name = ref(props.hit.title)
const description = ref(props.hit.description)

const selected = computed(() => versions.value.find(version => version.id === versionId.value) ?? null)

const versionItems = computed(() => versions.value.map(version => ({
  label: versionLabel(version),
  value: version.id,
  disabled: !version.supported,
})))

const { t } = useI18n()

const loaderLabel = computed(() => {
  const loader = selected.value?.loader
  return loader ? INSTANCE_TYPE_LABELS[loader] : t('search.install.loader_unknown')
})

const canInstall = computed(() =>
  !loading.value
  && !loadError.value
  && !creating.value
  && name.value.trim().length > 0
  && !!selected.value?.supported,
)

async function loadVersions() {
  loading.value = true
  loadError.value = null

  try {
    versions.value = await call('list_pack_versions', {
      provider: props.hit.provider,
      projectId: props.hit.projectId,
    })
    versionId.value = (versions.value.find(version => version.supported) ?? versions.value[0])?.id ?? ''
  }
  catch (e) {
    loadError.value = captureError(e, { code: 'NETWORK', context: { action: t('search.install.versions_action') } })
  }
  finally {
    loading.value = false
  }
}

onMounted(loadVersions)

async function packIcon(): Promise<string | undefined> {
  const url = props.hit.iconUrl
  if (!url) return undefined

  const saved = await safeRun(() => call('save_pack_icon', {
    provider: props.hit.provider,
    projectId: props.hit.projectId,
    url,
  }))

  return saved?.name
}

const install = async () => {
  if (!canInstall.value) return

  const version = selected.value
  const file = version?.file

  if (!version?.loader || !version.minecraftVersion || !file) return

  creating.value = true

  const icon = await packIcon()

  const created = await attempt(() => instanceStore.createInstance({
    id: v4(),
    name: name.value.trim(),
    description: description.value.trim(),
    type: version.loader!,
    minecraftVersion: version.minecraftVersion!,
    icon,
    version: 1,
    pack: {
      provider: props.hit.provider,
      projectId: props.hit.projectId,
      versionId: version.id,
      versionNumber: version.versionNumber || version.name,
      fileUrl: file.url,
      fileName: file.filename,
      fileSha1: file.hashes.sha1 ?? undefined,
      fileSize: file.size ?? undefined,
    },
  }))

  if (!created.ok) {
    creating.value = false
    return
  }

  await safeRun(() => instanceStore.installInstance(created.value.id), {
    code: 'NETWORK',
    context: { action: t('search.install.install_action'), instanceName: created.value.name },
  })

  creating.value = false
  emit('installed', created.value.id)
}
</script>

<template>
  <div>
    <div class="flex items-start gap-4 border-b border-line pb-5">
      <KitThumb
        :src="hit.iconUrl"
        :alt="hit.title"
      />

      <div class="min-w-0">
        <p class="font-mono text-micro uppercase tracking-caps text-acid">
          {{ PACK_PROVIDER_LABELS[hit.provider] }}
        </p>
        <h3 class="mt-1.5 truncate font-unbounded text-heading font-semibold tracking-heading text-fg">
          {{ hit.title }}
        </h3>
      </div>
    </div>

    <KitLoading
      v-if="loading"
      :label="$t('search.install.loading')"
    />

    <KitLoadError
      v-else-if="loadError"
      :error="loadError"
      class="mt-6"
      @retry="loadVersions"
    />

    <p
      v-else-if="!versions.length"
      class="mt-6 text-body leading-relaxed text-fg-muted"
    >
      {{ $t('search.install.no_versions') }}
    </p>

    <form
      v-else
      class="mt-6 space-y-8"
      @submit.prevent="install"
    >
      <div class="space-y-5">
        <KitField
          :label="$t('search.install.name')"
          for="pack-name"
        >
          <Input
            id="pack-name"
            v-model="name"
            size="lg"
            font="display"
          />
        </KitField>

        <KitField
          :label="$t('search.install.description')"
          for="pack-description"
        >
          <Input
            id="pack-description"
            v-model="description"
            :placeholder="$t('search.install.description_placeholder')"
          />
        </KitField>

        <KitField :label="$t('search.install.version')">
          <KitSelectMenu
            v-model="versionId"
            :items="versionItems"
            :search-placeholder="$t('search.install.version_search')"
          />
        </KitField>
      </div>

      <KitStatGrid>
        <KitStat
          label="Minecraft"
          :value="selected?.minecraftVersion ?? '-'"
        />
        <KitStat
          :label="$t('search.install.loader')"
          :value="loaderLabel"
        />
      </KitStatGrid>

      <KitNote
        v-if="selected && !selected.supported"
        icon="i-lucide-triangle-alert"
      >
        {{ $t('search.install.unsupported', { reason: unsupportedReason(selected) }) }}
      </KitNote>

      <KitNote
        v-else-if="selected?.blocked"
        icon="i-lucide-hand"
      >
        {{ $t('search.install.blocked_pack') }}
      </KitNote>

      <KitNote
        v-else-if="!hit.distributionAllowed"
        icon="i-lucide-triangle-alert"
      >
        {{ $t('search.install.blocked_files') }}
      </KitNote>

      <Button
        type="submit"
        size="xl"
        class="group/act w-full"
        :loading="creating"
        :disabled="!canInstall"
      >
        <Icon
          v-if="!creating"
          name="i-lucide-download"
          class="size-3.5 transition-transform duration-500 group-hover/act:translate-y-0.5"
        />
        {{ creating ? $t('search.install.creating') : $t('search.install.install') }}
      </Button>
    </form>
  </div>
</template>
