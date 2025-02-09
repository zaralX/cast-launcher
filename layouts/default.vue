<script setup lang="ts">

import MainHeader from "../components/main/MainHeader.vue";
</script>

<template>
  <div class="flex flex-col min-h-screen">
    <MainHeader/>
    <div class="flex-1 flex">
      <div class="flex-1">
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