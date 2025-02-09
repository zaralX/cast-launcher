<script setup lang="ts">

</script>

<template>
<div class="h-full">
  <el-scrollbar height="100%">
    index page
    <NuxtLink to="/test"> go to test </NuxtLink>
    <div class="grid grid-cols-3">
      <div v-for="i in 150">123</div>
    </div>
  </el-scrollbar>
</div>
</template>

<style scoped>

</style>