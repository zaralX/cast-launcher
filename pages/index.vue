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
<!--    <div class="w-full h-24 relative mb-4 shadow-lg border border-amber-400 rounded-lg bg-neutral-800 overflow-hidden">-->
<!--      <div class="absolute -left-8 -bottom-8 w-16 h-16 bg-amber-400 blur-3xl"></div>-->
<!--      <div class="absolute -right-8 -top-8 w-16 h-16 bg-amber-400 blur-3xl"></div>-->
<!--      <div class="absolute right-2 top-2 cursor-pointer text-amber-200 hover:text-amber-400"><i class="pi pi-times opacity-50 text-lg"></i></div>-->
<!--      <div class="w-full h-full p-2">-->
<!--        <p class="text-xl text-amber-400">Новый сезон!</p>-->
<!--      </div>-->
<!--    </div>-->
    <el-scrollbar height="100%">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
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