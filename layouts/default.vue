<script setup lang="ts">

import MainHeader from "../components/main/MainHeader.vue";
</script>

<template>
  <div class="flex flex-col">
    <div>
      <MainAppbar />
      <MainHeader/>
    </div>
    <div class="flex-1 flex">
      <div class="flex-1 h-[calc(100vh-3rem)]">
        <slot/>
      </div>
      <div class="w-32 bg-neutral-950">
        123
      </div>
    </div>
  </div>
</template>

<style scoped>

</style>