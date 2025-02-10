<script setup lang="ts">

import {check} from "@tauri-apps/plugin-updater";
import {relaunch} from "@tauri-apps/plugin-process";
import {ref} from "vue";

const message = ref("Проверка обновлений")
const loading = ref(true)
const newVersionData = ref({});

onMounted(async () => {
  await checkUpdates();
})

async function checkUpdates() {
  const update = await check();
  newVersionData.value = update;
  if (update) {
    message.value = "Обновление найдено"
    let downloaded = 0;
    let contentLength = 0;
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          contentLength = event.data.contentLength;
          message.value = "Получаем обновление"
          break;
        case 'Progress':
          downloaded += event.data.chunkLength;
          message.value = `Получаем обновление ${downloaded/contentLength*100}%`
          break;
        case 'Finished':
          message.value = `Обновление получено`
          break;
      }
    });

    message.value = `Обновление установлено<br>Перезапуск..`
    await relaunch();
  } else {
    message.value = `Последняя версия`
    loading.value = false;
  }
}
</script>

<template>
<div>
  <transition name="fade">
    <div v-if="loading" class="fixed w-full h-screen top-0 left-0 z-50 bg-black/[.8] flex justify-center items-center text-blue-500">
      <div class="animate-pulse flex flex-col items-center">
        <i class="pi pi-cloud-download text-5xl"></i>
        <p class="font-semibold">{{ message }}</p>
      </div>
    </div>
  </transition>
</div>
</template>

<style scoped>

</style>