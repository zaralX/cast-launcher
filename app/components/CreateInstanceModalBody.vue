<script setup lang="ts">
import type {InstanceType} from "~/types/instance";
import {$fetch} from "ofetch";

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

const instancesStore = useInstanceStore()

const id = ref("")
const name = ref("")
const description = ref("")
const instanceType = ref<InstanceType>("vanilla")
const minecraftVersion = ref<string>(minecraftVersionManifest.latest.release)
const fabricLoader = ref<string>(fabricLoaderVersions?.[0] ?? "latest")

const createInstance = async () => {
  await instancesStore.createInstance({
    id: id.value,
    name: name.value,
    description: description.value,
    type: instanceType.value,
    minecraftVersion: minecraftVersion.value,
    loaderVersion: instanceType.value == 'fabric' ? fabricLoader.value ?? 'latest' : undefined,
    version: 1,
    installed: false
  })
}
</script>

<template>
<div class="flex flex-col">
  <UInput v-model="id" placeholder="id" />
  <UInput v-model="name" placeholder="name" />
  <UInput v-model="description" placeholder="description" />
  <USelect v-model="instanceType" :items="['vanilla', 'fabric', 'forge']" />
  <USelect v-if="instanceType == 'fabric'" v-model="fabricLoader" :items="fabricLoaderVersions" />
  <USelect v-model="minecraftVersion" :items="minecraftVersions" />
  <UButton @click="createInstance">Создать</UButton>
</div>
</template>

<style scoped>

</style>