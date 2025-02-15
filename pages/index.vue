<script setup lang="ts">
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";
import {useLauncher} from "~/composables/useLauncher";

const {settings, packs, refreshPacks, updateClientState, clients} = useLauncher()

watch(settings, async (newSettings) => {
  if (newSettings) {
    await refreshPacks();
  }
}, { immediate: true });

const start = async (pack_id) => {
  await updateClientState(pack_id, "requesting");
  await invoke("run_pack", { packId: pack_id});
}
</script>

<template>
<div class="h-full">
  <el-scrollbar height="100%">
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 p-4">
      <div v-for="pack in packs" class="w-full gap-4 border rounded-lg border-neutral-700 shadow-lg bg-neutral-800">
        <div class="flex gap-4 p-2">
          <img src="/vite.svg" class="w-16 h-16" alt="">
          <div>
            <p class="text-lg font-medium">{{pack.cast_pack.name}}</p>
            <p class="text-xs">{{pack}}</p>
          </div>
          <div class="flex justify-end items-end">
            <el-button type="primary" plain size="small" @click="start(pack.cast_pack.pack_id)"><i class="pi pi-play text-xs"></i>  Играть</el-button>
          </div>
        </div>
      </div>
    </div>
  </el-scrollbar>
</div>
</template>

<style scoped>

</style>