<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
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

const appWindow = getCurrentWindow()

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
      class="pointer-events-none absolute -left-40 -top-40 h-[26rem] w-[26rem] rounded-full bg-acid/[0.07] blur-[120px]"
      aria-hidden="true"
    />

    <header
      data-tauri-drag-region
      class="relative z-30 flex h-11 shrink-0 items-stretch border-b border-line bg-ink-800/70"
    >
      <div class="pointer-events-none flex select-none items-center gap-3 pl-3 pr-6">
        <img
          src="/logo.svg"
          class="h-8 w-8"
          alt=""
        >
        <p class="font-unbounded text-[13px] font-semibold leading-none tracking-[-0.05em]">
          CAST<span class="text-acid">.</span>
        </p>
      </div>

      <div class="ml-auto mr-0 flex items-stretch">
        <UButton
          color="neutral"
          variant="ghost"
          :aria-label="$t('layout.window.minimize')"
          class="group h-11 w-11 justify-center text-fg-faint hover:bg-ink-600 hover:text-fg"
          @click="appWindow?.minimize()"
        >
          <span class="h-px w-3.5 bg-current transition-transform duration-300 group-hover:scale-x-75" />
        </UButton>

        <UButton
          color="neutral"
          variant="ghost"
          :aria-label="$t('layout.window.maximize')"
          class="group h-11 w-11 justify-center text-fg-faint hover:bg-ink-600 hover:text-fg"
          @click="appWindow?.toggleMaximize()"
        >
          <span class="size-2.5 border border-current transition-all duration-300 group-hover:size-3" />
        </UButton>

        <UButton
          color="neutral"
          variant="ghost"
          :aria-label="$t('layout.window.close')"
          class="group h-11 w-11 justify-center text-fg-faint hover:bg-red-500 hover:text-white"
          @click="appWindow?.close()"
        >
          <UIcon
            name="i-lucide-x"
            class="size-3.5 transition-transform duration-300 group-hover:rotate-90"
          />
        </UButton>
      </div>
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

      <p
        v-if="blocked"
        class="min-w-0 flex-1 truncate font-mono text-[10px] uppercase tracking-[0.2em] text-amber-400"
      >
        {{ $t('onboarding.account.required') }}
      </p>
      <span
        v-else
        class="flex-1"
      />

      <AppButton
        v-if="index > 0"
        tone="quiet"
        class="shrink-0 text-[10px] tracking-[0.18em]"
        :disabled="finishing"
        @click="back"
      >
        {{ $t('onboarding.back') }}
      </AppButton>

      <AppButton
        class="h-10 shrink-0 px-6 tracking-[0.2em]"
        :loading="finishing"
        :disabled="blocked || finishing"
        @click="next"
      >
        {{ nextLabel }}
      </AppButton>
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
