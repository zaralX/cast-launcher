<script setup lang="ts">
import {registerErrorSink} from "~/stores/error";
import type {ErrorSeverity} from "~/types/error";

const toaster = { position: 'bottom-right' }

const TOAST_COLOR: Record<ErrorSeverity, "error" | "warning" | "info"> = {
  error: "error",
  warning: "warning",
  info: "info"
}

const toast = useToast()
const errorCenterOpen = useErrorCenterOpen()

const unregister = registerErrorSink((entry) => {
  toast.add({
    title: entry.title,
    description: entry.hint ?? entry.message,
    icon: entry.icon,
    color: TOAST_COLOR[entry.severity],
    duration: entry.severity === "info" ? 4000 : 8000,
    actions: [{
      label: "Подробнее",
      color: "neutral",
      variant: "outline",
      onClick: () => {
        errorCenterOpen.value = true
      }
    }]
  })
})

onUnmounted(unregister)
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