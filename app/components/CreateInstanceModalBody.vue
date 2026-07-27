<script setup lang="ts">
import type {InstanceType} from "~/types/instance";
import {$fetch} from "ofetch";
import {XMLParser} from "fast-xml-parser";
import {v4} from "uuid";
import {LauncherError} from "~/types/error";

const emit = defineEmits<{ created: [] }>()

const instancesStore = useInstanceStore()

const loading = ref(true)
const loadError = ref<LauncherError | null>(null)
const creating = ref(false)

const minecraftVersions = ref<string[]>([])
const fabricLoaderVersions = ref<string[]>([])
const forgeVersions = ref<string[]>([])

const name = ref("")
const description = ref("")
const instanceType = ref<InstanceType>("vanilla")
const minecraftVersion = ref<string>("")
const fabricLoader = ref<string>("latest")
const forgeLoader = ref<string>("latest")

function compareForgeVersions(a: string, b: string) {
  const splitVersion = (v: string) => v.split(/[\.-]/).map(Number);
  const aParts = splitVersion(a);
  const bParts = splitVersion(b);

  for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
    const aNum = aParts[i] || 0;
    const bNum = bParts[i] || 0;
    if (aNum > bNum) return -1;
    if (aNum < bNum) return 1;
  }
  return 0;
}

const filteredForgeVersions = computed(() =>
    forgeVersions.value.filter((v: string) => v.startsWith(minecraftVersion.value))
)

const canCreate = computed(() =>
    !loading.value && !loadError.value && !creating.value && name.value.trim().length > 0
)

async function loadMetadata() {
  loading.value = true
  loadError.value = null

  try {
    const versionManifest = await $fetch<any>("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
    minecraftVersions.value = (versionManifest?.versions ?? [])
        .filter((v: any) => v.type == 'release')
        .map((v: any) => v.id)
    minecraftVersion.value = versionManifest?.latest?.release ?? minecraftVersions.value[0] ?? ""

    const fabricLoaders = await $fetch<{ version: string }[]>("https://meta.fabricmc.net/v2/versions/loader/")
    fabricLoaderVersions.value = (fabricLoaders ?? []).map(loader => loader.version)
    fabricLoader.value = fabricLoaderVersions.value[0] ?? "latest"

    const forgeXml = await $fetch<string>("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")
    const parsedXml = new XMLParser().parse(forgeXml)
    const versions = parsedXml?.metadata?.versioning?.versions?.version

    forgeVersions.value = Array.isArray(versions) ? [...versions] : []
    forgeVersions.value.sort(compareForgeVersions)
    forgeLoader.value = forgeVersions.value[0] ?? "latest"
  } catch (e) {
    loadError.value = captureError(e, {code: "NETWORK", context: {action: "Загрузка списка версий"}})
  } finally {
    loading.value = false
  }
}

onMounted(loadMetadata)

const createInstance = async () => {
  if (!canCreate.value) return

  creating.value = true

  const result = await attempt(() => instancesStore.createInstance({
    id: v4(),
    name: name.value.trim(),
    description: description.value.trim(),
    type: instanceType.value,
    minecraftVersion: minecraftVersion.value,
    loaderVersion: instanceType.value == 'fabric' ? fabricLoader.value : instanceType.value == 'forge' ? forgeLoader.value : undefined,
    version: 1,
    installed: false
  }))

  creating.value = false

  if (result.ok) emit("created")
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <div v-if="loading" class="flex items-center gap-2 py-6 justify-center text-sm text-zinc-400">
      <Icon name="i-lucide-loader-circle" class="animate-spin"/>
      Загрузка списка версий…
    </div>

    <UAlert
        v-else-if="loadError"
        color="error"
        variant="subtle"
        icon="i-lucide-wifi-off"
        :title="loadError.title"
        :description="loadError.hint ?? loadError.message"
        :actions="[{ label: 'Повторить', color: 'neutral', variant: 'outline', onClick: loadMetadata }]"
    />

    <template v-else>
      <UInput v-model="name" placeholder="Название сборки"/>
      <UInput v-model="description" placeholder="Описание"/>
      <USelect v-model="instanceType" :items="['vanilla', 'fabric', 'forge']"/>
      <USelect v-if="instanceType == 'fabric'" v-model="fabricLoader" :items="fabricLoaderVersions"/>
      <USelect v-if="instanceType == 'forge'" v-model="forgeLoader" :items="filteredForgeVersions"/>
      <USelect v-model="minecraftVersion" :items="minecraftVersions"/>
      <UButton :disabled="!canCreate" :loading="creating" @click="createInstance">Создать</UButton>
    </template>
  </div>
</template>

<style scoped>

</style>
