<script setup lang="ts">
import type { InstanceType } from '~/types/instance'
import { v4 } from 'uuid'
import type { NeoForgeRelease } from '~/types/backend'
import type { LauncherError } from '~/utils/error'

const emit = defineEmits<{ created: [] }>()

const { t } = useI18n()
const instancesStore = useInstanceStore()

const loading = ref(true)
const loadError = ref<LauncherError | null>(null)
const creating = ref(false)

const minecraftVersions = ref<string[]>([])
const fabricLoaderVersions = ref<string[]>([])
const forgeVersions = ref<string[]>([])
const neoforgeVersions = ref<NeoForgeRelease[]>([])

const name = ref('')
const description = ref('')
const instanceType = ref<InstanceType>('vanilla')
const minecraftVersion = ref<string>('')
const fabricLoader = ref<string>('latest')
const forgeLoader = ref<string>('latest')
const neoforgeLoader = ref<string>('')

const TYPES: { value: InstanceType, label: string, mark: string }[] = [
  { value: 'vanilla', label: 'Vanilla', mark: 'VA' },
  { value: 'fabric', label: 'Fabric', mark: 'FA' },
  { value: 'forge', label: 'Forge', mark: 'FO' },
  { value: 'neoforge', label: 'NeoForge', mark: 'NF' },
]

const filteredForgeVersions = computed(() =>
  forgeVersions.value.filter((v: string) => v.startsWith(minecraftVersion.value)),
)

const filteredNeoforgeVersions = computed(() =>
  neoforgeVersions.value
    .filter(release => release.minecraftVersion === minecraftVersion.value)
    .map(release => release.version),
)

const missingLoader = computed(() =>
  instanceType.value === 'neoforge' && filteredNeoforgeVersions.value.length === 0,
)

watch(filteredNeoforgeVersions, (versions) => {
  if (!versions.includes(neoforgeLoader.value)) neoforgeLoader.value = versions[0] ?? ''
}, { immediate: true })

const canCreate = computed(() =>
  !loading.value && !loadError.value && !creating.value && !missingLoader.value
  && name.value.trim().length > 0,
)

async function loadMetadata() {
  loading.value = true
  loadError.value = null

  try {
    const [manifest, fabric, forge, neoforge] = await Promise.all([
      call('list_minecraft_versions'),
      call('list_fabric_versions'),
      call('list_forge_versions'),
      call('list_neoforge_versions'),
    ])

    minecraftVersions.value = manifest.versions
      .filter(version => version.type === 'release')
      .map(version => version.id)
    minecraftVersion.value = manifest.latest.release ?? minecraftVersions.value[0] ?? ''

    fabricLoaderVersions.value = fabric
    fabricLoader.value = fabric[0] ?? 'latest'

    forgeVersions.value = forge
    forgeLoader.value = forge[0] ?? 'latest'

    neoforgeVersions.value = neoforge
  }
  catch (e) {
    loadError.value = captureError(e, { code: 'NETWORK', context: { action: t('create.versions_action') } })
  }
  finally {
    loading.value = false
  }
}

onMounted(loadMetadata)

function loaderVersion(): string | undefined {
  if (instanceType.value === 'fabric') return fabricLoader.value
  if (instanceType.value === 'forge') return forgeLoader.value
  if (instanceType.value === 'neoforge') return neoforgeLoader.value
  return undefined
}

const createInstance = async () => {
  if (!canCreate.value) return

  creating.value = true

  const result = await attempt(() => instancesStore.createInstance({
    id: v4(),
    name: name.value.trim(),
    description: description.value.trim(),
    type: instanceType.value,
    minecraftVersion: minecraftVersion.value,
    loaderVersion: loaderVersion(),
    version: 1,
  }))

  creating.value = false

  if (result.ok) emit('created')
}
</script>

<template>
  <div>
    <KitLoading
      v-if="loading"
      :label="$t('create.loading')"
    />

    <KitLoadError
      v-else-if="loadError"
      :error="loadError"
      @retry="loadMetadata"
    />

    <form
      v-else
      class="space-y-8"
      @submit.prevent="createInstance"
    >
      <KitField
        :label="$t('create.name')"
        for="instance-name"
      >
        <Input
          id="instance-name"
          v-model="name"
          :placeholder="$t('create.name_placeholder')"
        />
      </KitField>

      <KitField :label="$t('create.loader')">
        <RadioGroup
          v-model="instanceType"
          :aria-label="$t('create.loader')"
        >
          <RadioGroupItem
            v-for="type in TYPES"
            :key="type.value"
            :value="type.value"
            class="flex-col gap-1.5 py-4"
          >
            <span class="font-mono text-label tracking-[0.1em] transition-colors duration-300 group-data-[state=checked]:text-acid">
              {{ type.mark }}
            </span>
            <span class="font-unbounded text-body tracking-[-0.02em]">{{ type.label }}</span>
          </RadioGroupItem>
        </RadioGroup>
      </KitField>

      <div
        class="grid gap-5"
        :class="instanceType === 'vanilla' ? 'grid-cols-1' : 'grid-cols-2'"
      >
        <KitField label="Minecraft">
          <KitSelect
            v-model="minecraftVersion"
            :items="minecraftVersions"
          />
        </KitField>

        <KitField
          v-if="instanceType === 'fabric'"
          label="Fabric Loader"
        >
          <KitSelect
            v-model="fabricLoader"
            :items="fabricLoaderVersions"
          />
        </KitField>

        <KitField
          v-if="instanceType === 'forge'"
          label="Forge"
        >
          <KitSelect
            v-model="forgeLoader"
            :items="filteredForgeVersions"
          />
        </KitField>

        <KitField
          v-if="instanceType === 'neoforge'"
          label="NeoForge"
        >
          <KitSelect
            v-model="neoforgeLoader"
            :items="filteredNeoforgeVersions"
            :disabled="missingLoader"
            :placeholder="$t('create.no_builds')"
          />
        </KitField>
      </div>

      <p
        v-if="missingLoader"
        class="-mt-4 text-body leading-relaxed text-fg-muted"
      >
        {{ $t('create.missing_loader', { version: minecraftVersion }) }}
      </p>

      <Button
        type="submit"
        size="xl"
        class="group/act w-full"
        :loading="creating"
        :disabled="!canCreate"
      >
        <Icon
          v-if="!creating"
          name="i-lucide-plus"
          class="size-3.5 transition-transform duration-500 group-hover/act:rotate-90"
        />
        {{ creating ? $t('create.creating') : $t('create.create') }}
      </Button>
    </form>
  </div>
</template>
