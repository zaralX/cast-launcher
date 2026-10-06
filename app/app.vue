<script setup lang="ts">
const { t } = useI18n()
const toast = useAppToast()
const errorCenterOpen = useErrorCenterOpen()

useAppearance()
useLanguage()

const unregister = registerErrorSink((entry) => {
  toast.add({
    title: entry.title,
    description: entry.reason ?? entry.hint,
    icon: entry.icon,
    color: entry.severity,
    duration: entry.severity === 'info' ? 4000 : 8000,
    actions: [{
      label: t('common.details'),
      onClick: () => {
        errorCenterOpen.value = true
      },
    }],
  })
})

onUnmounted(unregister)
</script>

<template>
  <div class="grain relative max-w-screen max-h-screen overflow-hidden bg-ink-900 text-fg antialiased">
    <TooltipProvider :delay-duration="0">
      <NuxtLayout>
        <NuxtPage />
      </NuxtLayout>
      <Sonner />
    </TooltipProvider>
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
}

.page-leave-to {
  opacity: 0;
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
