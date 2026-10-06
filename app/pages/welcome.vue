<script setup lang="ts">
import type { ImportReport } from '~/types/import'

definePageMeta({
  layout: false,
})

type StepId = 'appearance' | 'account' | 'launcher'

const STEPS: StepId[] = ['appearance', 'account', 'launcher']

const { t } = useI18n()

const appStore = useAppStore()
const accountStore = useAccountStore()
const { accountConfig } = storeToRefs(accountStore)

const index = ref(0)
const finishing = ref(false)
const report = ref<ImportReport | null>(null)

const step = computed(() => STEPS[index.value]!)
const isLast = computed(() => index.value === STEPS.length - 1)

const hasAccount = computed(() => (accountConfig.value?.accounts?.length ?? 0) > 0)
const blocked = computed(() => step.value === 'account' && !hasAccount.value)

const nextLabel = computed(() => isLast.value
  ? t(finishing.value ? 'onboarding.finishing' : 'onboarding.finish')
  : t('onboarding.next'))

function next() {
  if (blocked.value) return

  if (isLast.value) {
    finish()
    return
  }

  index.value += 1
}

function back() {
  if (index.value > 0) index.value -= 1
}

async function finish() {
  if (finishing.value) return

  finishing.value = true

  const done = await attempt(() => appStore.finishOnboarding(), { code: 'CONFIG_ERROR' })

  finishing.value = false

  if (!done.ok) return

  trackEvent('onboarding_finished', {
    imported: report.value?.imported.length ?? 0,
    accounts: accountConfig.value?.accounts?.length ?? 0,
  })

  await navigateTo('/main')
}
</script>

<template>
  <div class="relative flex h-screen w-full flex-col overflow-hidden bg-ink-900 text-fg">
    <div
      class="pointer-events-none absolute -top-40 -left-40 size-[26rem] rounded-full bg-acid/[0.07] blur-[120px]"
      aria-hidden="true"
    />

    <header
      data-tauri-drag-region
      class="relative z-30 flex h-11 shrink-0 items-stretch justify-between border-b border-line bg-ink-800/70"
    >
      <AppLogo />
      <AppWindowControls />
    </header>

    <main class="relative z-20 min-h-0 min-w-0 flex-1">
      <Transition
        name="step"
        mode="out-in"
      >
        <div
          :key="step"
          class="h-full"
        >
          <OnboardingStepAppearance v-if="step === 'appearance'" />
          <OnboardingStepAccount v-else-if="step === 'account'" />
          <OnboardingStepLauncher
            v-else
            v-model="report"
          />
        </div>
      </Transition>
    </main>

    <footer class="relative z-30 flex shrink-0 items-center gap-5 border-t border-line bg-ink-800/70 px-6 py-4">
      <div class="flex shrink-0 items-center gap-1.5">
        <span
          v-for="(item, i) in STEPS"
          :key="item"
          class="h-px w-6 transition-colors duration-500"
          :class="i <= index ? 'bg-acid' : 'bg-line'"
        />
      </div>

      <KitStatus
        v-if="blocked"
        tone="warning"
        dot="static"
        class="min-w-0 flex-1"
      >
        {{ $t('onboarding.account.required') }}
      </KitStatus>
      <span
        v-else
        class="flex-1"
      />

      <Button
        v-if="index > 0"
        variant="quiet"
        :disabled="finishing"
        @click="back"
      >
        {{ $t('onboarding.back') }}
      </Button>

      <Button
        size="lg"
        :loading="finishing"
        :disabled="blocked || finishing"
        @click="next"
      >
        {{ nextLabel }}
      </Button>
    </footer>
  </div>
</template>

<style scoped>
.step-enter-active,
.step-leave-active {
  transition: opacity 0.16s ease;
}

.step-enter-from,
.step-leave-to {
  opacity: 0;
}
</style>
