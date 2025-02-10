<script setup lang="ts">
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";
import {useLauncher} from "~/composables/useLauncher";

const {settings, packs, refreshPacks} = useLauncher()

watch(settings, async (newSettings) => {
  if (newSettings) {
    await refreshPacks();
  }
}, { immediate: true });

const start = async (pack_id) => {
  await invoke("run_pack", { packId: pack_id});
}

const createPack = async () => {
  await invoke("create_pack", newPack.value);
}

const newPack = ref({
  packId: "",
  version: "",
  versionType: "vanilla"
});
</script>

<template>
<div class="h-full">
  <div class="flex">
    <el-input v-model="newPack.packId" placeholder="Pack id" />
    <el-input v-model="newPack.version" placeholder="Minecraft version" />
    <el-select v-model="newPack.versionType" placeholder="Version type">
      <el-option
          key="vanilla"
          label="Vanilla"
          value="vanilla"
      />
    </el-select>
    <el-button @click="createPack">CREATE PACK</el-button>
  </div>
  <el-scrollbar height="100%">
    <div class="grid grid-cols-3">
      <div v-for="pack in packs">
        {{ pack }}
        <el-button @click="start(pack.cast_pack.pack_id)">Start</el-button>
      </div>
    </div>
  </el-scrollbar>
</div>
</template>

<style scoped>

</style>