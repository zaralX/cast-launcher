<script setup lang="ts">
const { t } = useI18n()

const loading = ref(true)
const steps = computed(() => [
  t('boot.steps.waiting'),
  t('boot.steps.connecting'),
  t('boot.steps.updates'),
  t('boot.steps.done'),
])
const currentStep = ref(1)
const appStore = useAppStore()

onMounted(async () => {
  currentStep.value = 2

  await safeRun(() => useLauncherEvents(), { code: 'CONFIG_ERROR' })
  currentStep.value += 1

  if (appStore.config?.launcher?.auto_update) {
    await safeRun(() => appStore.updateApp(), { code: 'UPDATE_FAILED' })
  }
  currentStep.value += 1

  loading.value = false
  navigateTo(appStore.needsOnboarding ? '/welcome' : '/main')
})
</script>

<template>
  <div class="h-screen w-full">
    <AppLoadingScreen
      v-model="currentStep"
      :steps="steps"
    />
  </div>
</template>
