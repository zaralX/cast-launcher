<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import {useLauncher} from "~/composables/useLauncher";
import {invoke} from "@tauri-apps/api/core";

const {settings} = useLauncher()

const appWindow = getCurrentWindow();
const selected = ref(null)

let settingsUpdates = 0
watch(settings, value => {
  selected.value = settings.value.profiles.find(p => p.selected)?.username
  settingsUpdates++;
}, {deep: true})

watch(selected, async (value) => {
  if (settingsUpdates == 1) return;
  settings.value.profiles = settings.value.profiles.filter((p) => {p.selected = p.username === selected.value; return true})
  await invoke("save_settings", { settings: settings.value })
})

const openedUserMenu = ref(false)
const userMenuButton = ref(null);
const menuRef = ref(null);

const handleClickOutside = (event) => {
  if (userMenuButton.value && !userMenuButton.value.contains(event.target) && menuRef.value && !menuRef.value.contains(event.target)) {
    openedUserMenu.value = false;
  }
};


onMounted(() => {
  document.addEventListener('click', handleClickOutside);
});

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside);
});
</script>

<template>
<div data-tauri-drag-region class="bg-neutral-800 h-12 w-full select-none flex">
  <div class="flex items-center pl-2 relative">
    <div ref="userMenuButton" @click="openedUserMenu = !openedUserMenu" class="cursor-pointer w-10 h-10 hover:opacity-75 active:scale-110 transition duration-200">
      <img src="/default_skin_face.png" class="w-10 h-10 rounded-xl pointer-events-none" alt="">
    </div>
    <transition name="fade-w">
      <div v-if="openedUserMenu" ref="menuRef" class="absolute w-52 top-12 bg-neutral-950/[.75] p-2 rounded-lg border border-neutral-700/[.25] backdrop-blur-sm">
        <el-select v-model="selected" class="mb-2" size="default" placeholder="Выбрать аккаунт">
          <template #label>
            <div class="flex gap-2 items-center w-full">
              <img src="/default_skin_face.png" class="w-6 h-6 rounded-lg" alt="">
              <p>{{selected}}</p>
            </div>
          </template>
          <el-option v-for="profile in settings.profiles" :value="profile.username">
            <div class="flex gap-2 items-center w-full">
              <img src="/default_skin_face.png" class="w-6 h-6 rounded-lg" alt="">
              <p>{{profile.username}}</p>
            </div>
          </el-option>
        </el-select>
        <NuxtLink to="/settings/profiles">
          <el-button class="w-full" size="small" type="primary" text bg>
            <div class="flex gap-2">
              <i class="pi pi-plus"></i> <p>Добавить аккаунт</p>
            </div>
          </el-button>
        </NuxtLink>
      </div>
    </transition>
  </div>
  <div style="-webkit-app-region: drag"  class="flex-1 flex items-center px-2">

  </div>
  <div class="flex">
    <div @click="appWindow.minimize()" class="flex justify-center items-center w-10 transition-all duration-200 hover:bg-neutral-700 cursor-pointer"><i class="pi pi-minus text-sm"></i></div>
    <div @click="appWindow.toggleMaximize()" class="flex justify-center items-center w-10 transition-all duration-200 hover:bg-neutral-700 cursor-pointer"><i class="pi pi-window-maximize text-sm"></i></div>
    <div @click="appWindow.close()" class="flex justify-center items-center w-10 transition-all duration-200 hover:bg-red-500 hover:text-black cursor-pointer"><i class="pi pi-times text-sm"></i></div>
  </div>
</div>
</template>

<style scoped>

</style>