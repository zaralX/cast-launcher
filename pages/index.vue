<script setup>
import {invoke} from "@tauri-apps/api/core";
import {useLauncher} from "~/composables/useLauncher";

const {settings, packs, refreshPacks, updateClientState, clients} = useLauncher()

onMounted(async () => {
  await refreshPacks();
  console.log(packs.value);
})

const start = async (pack_id) => {
  await invoke("run_pack", {id: pack_id});
}

const install = async (pack_id) => {
  await invoke("install_pack", {id: pack_id});
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
              <p class="text-lg font-medium">{{ pack?.cast_pack?.name }}</p>
              <p class="text-xs">{{ pack }}</p>
            </div>
            <div class="flex justify-end items-end">
              <el-button v-if="pack['cast-pack']?.installed" type="primary" plain size="small" @click="start(pack['cast-pack']?.id)"><i
                  class="pi pi-play text-xs"></i>  Играть
              </el-button>
              <el-button v-else type="warning" plain size="small" @click="install(pack['cast-pack']?.id)"><i
                  class="pi pi-download text-xs"></i>  Установить
              </el-button>
            </div>
          </div>
        </div>
      </div>
    </el-scrollbar>
  </div>
</template>

<style scoped>

</style>