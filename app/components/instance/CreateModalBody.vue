<script setup lang="ts">
import type { InstanceType } from '~/types/instance'
import { INSTANCE_TYPE_LABELS, INSTANCE_TYPE_MARKS, INSTANCE_TYPES } from '~/types/instance'
import { v4 } from 'uuid'

const emit = defineEmits<{ created: [] }>()

const instancesStore = useInstanceStore()
const versions = useGameVersions()
const { loading, loadError, minecraft: minecraftVersions } = versions

const creating = ref(false)

const name = ref('')
const description = ref('')
const instanceType = ref<InstanceType>('vanilla')
const minecraftVersion = ref<string>('')
const fabricLoader = ref<string>('latest')
const forgeLoader = ref<string>('latest')
const neoforgeLoader = ref<string>('')

const fabricLoaderVersions = computed(() => versions.loaderVersions('fabric', minecraftVersion.value))
const filteredForgeVersions = computed(() => versions.loaderVersions('forge', minecraftVersion.value))
const filteredNeoforgeVersions = computed(() => versions.loaderVersions('neoforge', minecraftVersion.value))

const missingLoader = computed(() =>
  instanceType.value === 'neoforge' && filteredNeoforgeVersions.value.length === 0,
)

watch(versions.latestRelease, (release) => {
  minecraftVersion.value = release
})

watch(fabricLoaderVersions, (list) => {
  fabricLoader.value = list[0] ?? 'latest'
})

watch(filteredForgeVersions, (list) => {
  if (!list.includes(forgeLoader.value)) forgeLoader.value = list[0] ?? 'latest'
})

watch(filteredNeoforgeVersions, (list) => {
  if (!list.includes(neoforgeLoader.value)) neoforgeLoader.value = list[0] ?? ''
}, { immediate: true })

const canCreate = computed(() =>
  !loading.value && !loadError.value && !creating.value && !missingLoader.value
  && name.value.trim().length > 0,
)

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
      @retry="versions.load"
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
        />
      </KitField>

      <KitField :label="$t('create.loader')">
        <RadioGroup
          v-model="instanceType"
          :aria-label="$t('create.loader')"
        >
          <RadioGroupItem
            v-for="type in INSTANCE_TYPES"
            :key="type"
            :value="type"
            class="flex-col gap-1.5 py-4"
          >
            <span class="font-mono text-caption tracking-[0.1em] transition-colors duration-300 group-data-[state=checked]:text-accent">
              {{ INSTANCE_TYPE_MARKS[type] }}
            </span>
            <span class="font-unbounded text-body tracking-[-0.02em]">{{ INSTANCE_TYPE_LABELS[type] }}</span>
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
        variant="primary"
        type="submit"
        size="lg"
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
