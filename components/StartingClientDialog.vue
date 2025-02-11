<script setup lang="ts">
import {useLauncher} from "~/composables/useLauncher";
import {listen} from "@tauri-apps/api/event";
import {onUnmounted} from "vue";

const {settings, packs, clients} = useLauncher()

let unlisten = null;
const selected = ref(null)
const launchInfo = ref(null)

onMounted(async () => {
  unlisten = await listen("launching", (event) => {
    selected.value.state = event.payload?.state;
    launchInfo.value = event.payload;
  });
  updateSelected()
})

onUnmounted(() => {
  if (unlisten) unlisten();
})

watch(() => clients.value, () => {
  updateSelected()
}, {deep: true})

const updateSelected = () => {
  selected.value = clients.value.find((client) =>
      ["error",
        "requesting",
        "init",
        "versions",
        "starting",
        "version_info",
        "downloading_jar",
        "downloading_assets",
        "downloading_libraries",
        "installed",
          "starting"
      ].includes(client.state)
  );
}
</script>

<template>
  <transition name="fade">
    <div v-if="selected"
         class="fixed w-full h-screen top-0 left-0 z-50 bg-black/[.8] flex justify-center items-center text-blue-500">
      <div class="bg-neutral-900 rounded-lg p-2 border border-neutral-800 shadow-lg">
        <p class="text-white text-lg font-medium">Запуск клиента {{ selected?.pack_id }}</p>
        <p class="text-white text-lg">{{ selected }}</p>
        <p class="text-white text-lg">{{ launchInfo }}</p>
        <div class="flex justify-center mt-4">
          <el-button v-if="['starting', 'error'].includes(selected?.state)" type="warning" plain @click="selected = null">Закрыть</el-button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>

</style>