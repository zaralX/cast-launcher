<script setup lang="ts">
const toaster = { position: 'top-right' }
</script>

<template>
  <UApp :toaster="toaster">
    <NuxtLayout>
      <NuxtPage/>
    </NuxtLayout>
  </UApp>
</template>
