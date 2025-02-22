<script setup>

import {invoke} from "@tauri-apps/api/core";
import {useLauncher} from "~/composables/useLauncher";

const {settings, packs, refreshPacks, updateClientState, clients} = useLauncher()

const nzPacks = ref([{
  id: 'debthunt-0.4',
  name: 'DebtHunt',
  banner: '/image.png',
  version: '0.4',
  minecraft: {
    version: '1.12.2',
    installer: 'forge',
    forge: '14.23.5.2859'
  }
}])

const start = async (pack_id) => {
  await updateClientState(pack_id, "requesting");
  await invoke("run_zproject", {packId: pack_id, javaPath: 'C:\\Program Files\\Java\\jdk1.8.0_333\\bin\\java.exe'});
}
</script>

<template>
<div>
  <h2>Nazzy packs</h2>
  <div class="grid grid-cols-2">
    <div v-for="pack in nzPacks" class="w-full gap-4 border rounded-lg border-neutral-700 shadow-lg bg-neutral-800 overflow-hidden">
      <div class="flex flex-col">
        <div style="background-image: url('/image.png')" class="w-full h-20 bg-cover"/>
        <div class="p-2">
          <p class="text-lg font-medium">{{ pack.name }} v{{ pack.version }}</p>
        </div>
        <div class="flex justify-end items-end p-2">
          <el-button type="primary" plain size="small" @click="start(pack?.id)"><i
              class="pi pi-play text-xs"></i>  Играть
          </el-button>
        </div>
      </div>
    </div>
  </div>
</div>
</template>

<style scoped>

</style>