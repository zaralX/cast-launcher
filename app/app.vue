<script setup lang="ts">
const toaster = { position: 'bottom-right' }
</script>

<template>
  <div class="max-w-screen max-h-screen overflow-hidden">
    <UApp :toaster="toaster">
      <NuxtLayout>
        <NuxtPage/>
      </NuxtLayout>
    </UApp>
  </div>
</template>

<style>
.layout-enter-active,
.layout-leave-active {
  transition: all 0.25s ease-out;
}

.layout-enter-from {
  filter: grayscale(1) blur(0.5rem);
  transform: scale(110%);
  opacity: 0;
}

.layout-leave-to {
  filter: grayscale(1) blur(1rem);
  transform: translateY(-32px) scale(90%);
  opacity: 0;
}
</style>