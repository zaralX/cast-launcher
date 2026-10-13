<script setup lang="ts">
const { t } = useI18n()

const appStore = useAppStore()

const steps = computed(() => [
  t('boot.steps.waiting'),
  t('boot.steps.connecting'),
  t('boot.steps.done'),
])
const currentStep = ref(1)

onMounted(async () => {
  currentStep.value = 2

  await safeRun(() => useLauncherEvents(), { code: 'CONFIG_ERROR' })
  currentStep.value += 1

  await navigateTo(appStore.needsOnboarding ? '/welcome' : '/main')

  appStore.checkUpdate().catch(error => writeErrorLog(toLauncherError(error, 'UPDATE_FAILED')))
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
