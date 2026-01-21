<script setup lang="ts">
const toaster = { position: 'bottom-right' }
</script>

<template>
  <UApp :toaster="toaster">
    <NuxtLayout>
      <NuxtPage/>
    </NuxtLayout>
  </UApp>
</template>
