<script setup lang="ts">
import {registerErrorSink} from "~/stores/error";
import type {ErrorSeverity} from "~/types/error";

const toaster = { position: 'bottom-right' } as const

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
  <div class="grain relative max-w-screen max-h-screen overflow-hidden bg-ink-900 text-fg antialiased">
    <UApp :toaster="toaster">
      <NuxtLayout>
        <NuxtPage/>
      </NuxtLayout>
    </UApp>
  </div>
</template>

<style>
.layout-enter-active {
  transition: opacity 0.4s cubic-bezier(0.16, 1, 0.3, 1),
              transform 0.4s cubic-bezier(0.16, 1, 0.3, 1),
              filter 0.4s cubic-bezier(0.16, 1, 0.3, 1);
}

.layout-leave-active {
  transition: opacity 0.22s ease-in, transform 0.22s ease-in, filter 0.22s ease-in;
}

.layout-enter-from {
  opacity: 0;
  transform: scale(1.03);
  filter: blur(10px) saturate(0);
}

.layout-leave-to {
  opacity: 0;
  transform: translateY(-18px) scale(0.985);
  filter: blur(8px) saturate(0);
}

.page-enter-active {
  transition: opacity 0.32s cubic-bezier(0.16, 1, 0.3, 1),
              transform 0.32s cubic-bezier(0.16, 1, 0.3, 1);
}

.page-leave-active {
  transition: opacity 0.14s ease-in, transform 0.14s ease-in;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

@media (prefers-reduced-motion: reduce) {
  .layout-enter-from,
  .layout-leave-to,
  .page-enter-from,
  .page-leave-to {
    transform: none;
    filter: none;
  }
}
</style>
