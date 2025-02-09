<script setup lang="ts">

import MainHeader from "../components/main/MainHeader.vue";
import {listen} from "@tauri-apps/api/event";
import {onUnmounted, ref} from "vue";

let unlisten = null;
const currentDownloading = ref(null);

onMounted(async () => {
  unlisten = await listen("downloading", (event) => {
    currentDownloading.value = event.payload;
  });
})

onUnmounted(() => {
  if (unlisten) unlisten();
})
</script>

<template>
  <div class="flex flex-col">
    <MainHeader/>
    <div class="flex-1 flex">
      <div class="flex-1 h-[calc(100vh-3rem)]">
        <slot/>
      </div>
      <div class="w-32 bg-neutral-950">
        123
      </div>
    </div>

    <div class="fixed bottom-0 left-0 bg-blue-500 font-bold">{{currentDownloading}}</div>
  </div>
</template>

<style scoped>

</style>