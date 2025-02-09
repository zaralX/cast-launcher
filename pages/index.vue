<script setup lang="ts">
import {invoke} from "@tauri-apps/api/core";

const packs = ref([])

onMounted(async () => {
  packs.value = await invoke("get_packs", { launcherDir: "D:/RustProjects/cast-launcher/test" });
})
</script>

<template>
<div class="h-full">
  <el-scrollbar height="100%">
    index page
    <NuxtLink to="/test"> go to test </NuxtLink>
    <div class="grid grid-cols-3">
      <div v-for="pack in packs">
        {{ pack }}
        <el-button>Start</el-button>
      </div>
    </div>
  </el-scrollbar>
</div>
</template>

<style scoped>

</style>