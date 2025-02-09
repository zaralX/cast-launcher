<script setup lang="ts">
import {invoke} from "@tauri-apps/api/core";

const packs = ref([])

onMounted(async () => {
  packs.value = await invoke("get_packs", { launcherDir: "D:/RustProjects/cast-launcher/test" });
})

const start = async (pack_id) => {
  await invoke("run_pack", { packId: pack_id });
}
</script>

<template>
<div class="h-full">
  {{settings}}
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