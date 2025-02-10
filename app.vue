<script setup>
import {useLauncher} from "~/composables/useLauncher.js";
import UpdateDialog from "~/components/UpdateDialog.vue";

const colorMode = useColorMode()
const { initData } = useLauncher();

onMounted(async () => {
  await initData()
})
</script>

<template>
  <div class="bg-main-light dark:bg-neutral-900 transition-all duration-500 min-h-screen font-montserrat overflow-hidden">
    <UpdateDialog />
    <NuxtLayout>
      <NuxtPage />
    </NuxtLayout>
  </div>
</template>

<style>
.page-enter-active,
.page-leave-active {
  transition: all 0.4s;
}
.page-enter-from,
.page-leave-to {
  opacity: 0;
  //filter: blur(1rem);
}

.layout-enter-active,
.layout-leave-active {
  transition: all 0.4s;
}
.layout-enter-from,
.layout-leave-to {
  opacity: 0;
  filter: blur(1rem);
}
</style>