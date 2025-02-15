<script setup lang="ts">

</script>

<template>
  <div class="bg-neutral-950 h-12 grid grid-cols-5">
    <NuxtLink to="/">
      <div class="w-full h-full flex justify-center items-center">
        <p>Ваши сборки</p>
      </div>
    </NuxtLink>
    <NuxtLink to="/test">
      <div class="w-full h-full flex justify-center items-center">
        <p>Тест</p>
      </div>
    </NuxtLink>
    <NuxtLink to="/settings">
      <div class="w-full h-full flex justify-center items-center">
        <p>Настройки</p>
      </div>
    </NuxtLink>
  </div>
</template>

<style scoped>

</style>