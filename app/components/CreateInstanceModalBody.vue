<script setup lang="ts">
import type {InstanceType} from "~/types/instance";
import {$fetch} from "ofetch";
import {XMLParser} from "fast-xml-parser";
import {v4} from "uuid";

const minecraftVersionManifest = await $fetch("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
const minecraftVersions = ref(minecraftVersionManifest.versions.filter((v: any) => v.type == 'release').map((v: any) => v.id))

const fabricLoaders: {
  separator: string,
  build: number,
  maven: string,
  version: string,
  stable: boolean
}[] = await $fetch("https://meta.fabricmc.net/v2/versions/loader/")
const fabricLoaderVersions = fabricLoaders.map(loader => loader.version)

const forgeXml: string = await $fetch("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")
const parsedXml = new XMLParser().parse(forgeXml)

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

const forgeVersions = parsedXml.metadata.versioning.versions.version
forgeVersions.sort(compareForgeVersions)

const instancesStore = useInstanceStore()

const name = ref("")
const description = ref("")
const instanceType = ref<InstanceType>("vanilla")
const minecraftVersion = ref<string>(minecraftVersionManifest.latest.release)
const fabricLoader = ref<string>(fabricLoaderVersions?.[0] ?? "latest")
const forgeLoader = ref<string>(forgeVersions?.[0] ?? "latest")

const filteredForgeVersions = computed(() => {
  return forgeVersions.filter((v: string) => v.startsWith(minecraftVersion.value))
})

const createInstance = async () => {
  await instancesStore.createInstance({
    id: v4(),
    name: name.value,
    description: description.value,
    type: instanceType.value,
    minecraftVersion: minecraftVersion.value,
    loaderVersion: instanceType.value == 'fabric' ? fabricLoader.value : instanceType.value == 'forge' ? forgeLoader.value : undefined,
    version: 1,
    installed: false
  })
}
</script>

<template>
<div class="flex flex-col">
  <UInput v-model="name" placeholder="name" />
  <UInput v-model="description" placeholder="description" />
  <USelect v-model="instanceType" :items="['vanilla', 'fabric', 'forge']" />
  <USelect v-if="instanceType == 'fabric'" v-model="fabricLoader" :items="fabricLoaderVersions" />
  <USelect v-if="instanceType == 'forge'" v-model="forgeLoader" :items="filteredForgeVersions" />
  <USelect v-model="minecraftVersion" :items="minecraftVersions" />
  <UButton @click="createInstance">Создать</UButton>
</div>
</template>

<style scoped>

</style>