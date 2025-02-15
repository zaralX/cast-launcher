<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
console.log(appWindow);
</script>

<template>
<div data-tauri-drag-region class="bg-neutral-800 h-12 w-full select-none flex">
  <div style="-webkit-app-region: drag"  class="flex-1"></div>
  <div class="flex">
    <div @click="appWindow.minimize()" class="flex justify-center items-center w-10 transition-all duration-200 hover:bg-neutral-700 cursor-pointer"><i class="pi pi-minus text-sm"></i></div>
    <div @click="appWindow.toggleMaximize()" class="flex justify-center items-center w-10 transition-all duration-200 hover:bg-neutral-700 cursor-pointer"><i class="pi pi-window-maximize text-sm"></i></div>
    <div @click="appWindow.close()" class="flex justify-center items-center w-10 transition-all duration-200 hover:bg-red-500 hover:text-black cursor-pointer"><i class="pi pi-times text-sm"></i></div>
  </div>
</div>
</template>

<style scoped>

</style>