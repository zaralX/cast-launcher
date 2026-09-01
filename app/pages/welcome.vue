<script setup lang="ts">
import {storeToRefs} from "pinia"
import {getCurrentWindow} from "@tauri-apps/api/window"
import {useAccountStore} from "~/stores/account"
import {useAppStore} from "~/stores/app"
import {trackEvent} from "~/composables/useTelemetry"
import type {ImportReport} from "~/types/import"

definePageMeta({
  layout: false
})

type StepId = "language" | "appearance" | "privacy" | "account" | "import" | "done"

interface Step {
  id: StepId
  icon: string
}

const STEPS: Step[] = [
  {id: "language", icon: "i-lucide-languages"},
  {id: "appearance", icon: "i-lucide-palette"},
  {id: "privacy", icon: "i-lucide-shield-check"},
  {id: "account", icon: "i-lucide-user-round"},
  {id: "import", icon: "i-lucide-import"},
  {id: "done", icon: "i-lucide-rocket"}
]

const {t} = useI18n()

const appStore = useAppStore()
const accountStore = useAccountStore()
const {accountConfig} = storeToRefs(accountStore)

const appWindow = getCurrentWindow()

const index = ref(0)
const forward = ref(true)
const finishing = ref(false)
const report = ref<ImportReport | null>(null)
const furthest = ref(0)

const step = computed(() => STEPS[index.value]!)
const isLast = computed(() => index.value === STEPS.length - 1)
const percent = computed(() => Math.round(((index.value + 1) / STEPS.length) * 100))

const hasAccount = computed(() => (accountConfig.value?.accounts?.length ?? 0) > 0)

const blocked = computed(() => step.value.id === "account" && !hasAccount.value)

const nextLabel = computed(() => {
  if (isLast.value) return t(finishing.value ? "onboarding.finishing" : "onboarding.finish")
  if (step.value.id === "import" && !report.value) return t("onboarding.skip")
  return t("onboarding.next")
})

function go(to: number) {
  const clamped = Math.min(Math.max(to, 0), STEPS.length - 1)
  if (clamped === index.value) return

  forward.value = clamped > index.value
  index.value = clamped
  furthest.value = Math.max(furthest.value, clamped)
}

function next() {
  if (blocked.value) return

  if (isLast.value) {
    finish()
    return
  }

  go(index.value + 1)
}

const back = () => go(index.value - 1)

const ACCOUNT_STEP = STEPS.findIndex(item => item.id === "account")

function jump(to: number) {
  if (to > furthest.value) return
  if (to > ACCOUNT_STEP && !hasAccount.value) return

  go(to)
}

async function finish() {
  if (finishing.value) return

  finishing.value = true

  const done = await attempt(() => appStore.finishOnboarding(), {code: "CONFIG_ERROR"})

  finishing.value = false

  if (!done.ok) return

  trackEvent("onboarding_finished", {
    imported: report.value?.imported.length ?? 0,
    accounts: accountConfig.value?.accounts?.length ?? 0
  })

  await navigateTo("/main")
}

const transition = computed(() => forward.value ? "step-forward" : "step-back")
</script>

<template>
  <div class="relative flex h-screen w-full flex-col overflow-hidden bg-ink-900 text-fg">
    <OnboardingBackdrop/>

    <header data-tauri-drag-region class="relative z-30 flex h-11 shrink-0 items-stretch border-b border-line bg-ink-800/70 backdrop-blur-sm">
      <div class="pointer-events-none flex select-none items-center gap-3 pl-3 pr-6">
        <img src="/logo.svg" class="h-8 w-8" alt=""/>
        <p class="font-unbounded text-[13px] font-semibold leading-none tracking-[-0.05em]">
          CAST<span class="text-acid">.</span>
        </p>
      </div>

      <div class="flex items-stretch ml-auto mr-0">
        <UButton
            color="neutral"
            variant="ghost"
            :aria-label="$t('layout.window.minimize')"
            class="group h-11 w-11 justify-center text-fg-faint hover:bg-ink-600 hover:text-fg"
            @click="appWindow?.minimize()"
        >
          <span class="h-px w-3.5 bg-current transition-transform duration-300 group-hover:scale-x-75"/>
        </UButton>

        <UButton
            color="neutral"
            variant="ghost"
            :aria-label="$t('layout.window.maximize')"
            class="group h-11 w-11 justify-center text-fg-faint hover:bg-ink-600 hover:text-fg"
            @click="appWindow?.toggleMaximize()"
        >
          <span class="size-2.5 border border-current transition-all duration-300 group-hover:size-3"/>
        </UButton>

        <UButton
            color="neutral"
            variant="ghost"
            :aria-label="$t('layout.window.close')"
            class="group h-11 w-11 justify-center text-fg-faint hover:bg-red-500 hover:text-white"
            @click="appWindow?.close()"
        >
          <UIcon name="i-lucide-x" class="size-3.5 transition-transform duration-300 group-hover:rotate-90"/>
        </UButton>
      </div>
    </header>

    <div class="relative z-20 flex min-h-0 flex-1">
      <aside class="hidden w-[14.5rem] shrink-0 flex-col justify-between border-r border-line bg-ink-800/40 py-7 lg:flex">
        <ol class="space-y-1">
          <li v-for="(item, i) in STEPS" :key="item.id">
            <button
                type="button"
                class="group relative flex w-full items-center gap-3.5 px-6 py-2.5 text-left transition-colors duration-300"
                :class="[
                  i === index ? 'text-fg' : i <= furthest ? 'text-fg-muted hover:text-fg' : 'text-fg-faint/45',
                  i <= furthest ? 'cursor-pointer' : 'cursor-default'
                ]"
                :disabled="i > furthest"
                @click="jump(i)"
            >
              <span
                  class="absolute left-0 top-1/2 w-px -translate-y-1/2 bg-acid transition-all duration-500 ease-deck"
                  :class="i === index ? 'h-7 opacity-100' : 'h-0 opacity-0 group-hover:h-3.5 group-hover:opacity-60'"
              />

              <span
                  class="grid size-6 shrink-0 place-items-center border transition-all duration-500 ease-deck"
                  :class="i < index
                    ? 'border-acid/50 text-acid'
                    : i === index
                      ? 'border-acid bg-acid text-on-acid'
                      : 'border-line'"
              >
                <UIcon v-if="i < index" name="i-lucide-check" class="size-3"/>
                <UIcon v-else :name="item.icon" class="size-3"/>
              </span>

              <span class="min-w-0 flex-1 truncate font-mono text-[10px] uppercase tracking-[0.2em]">
                {{ $t(`onboarding.${item.id}.name`) }}
              </span>
            </button>
          </li>
        </ol>

        <div class="px-6">
          <p class="font-mono text-[9px] uppercase leading-relaxed tracking-[0.24em] text-fg-faint/60">
            {{ $t('onboarding.rail_hint') }}
          </p>
        </div>
      </aside>

      <main class="relative min-h-0 min-w-0 flex-1">
        <Transition :name="transition" mode="out-in">
          <div :key="step.id" class="h-full">
            <OnboardingStepLanguage v-if="step.id === 'language'"/>
            <OnboardingStepAppearance v-else-if="step.id === 'appearance'"/>
            <OnboardingStepPrivacy v-else-if="step.id === 'privacy'"/>
            <OnboardingStepAccount v-else-if="step.id === 'account'"/>
            <OnboardingStepImport v-else-if="step.id === 'import'" v-model="report"/>
            <OnboardingStepDone v-else :report="report"/>
          </div>
        </Transition>
      </main>
    </div>

    <footer class="relative z-30 shrink-0 border-t border-line bg-ink-800/70 backdrop-blur-sm">
      <div class="h-px w-full bg-line">
        <div
            class="h-px bg-acid transition-[width] duration-700 ease-deck"
            :style="{ width: `${percent}%` }"
        />
      </div>

      <div class="flex items-center gap-4 px-6 py-4">
        <AppButton
            tone="quiet"
            class="text-[10px] tracking-[0.18em]"
            icon="i-lucide-arrow-left"
            :disabled="index === 0 || finishing"
            @click="back"
        >
          {{ $t('onboarding.back') }}
        </AppButton>

        <p class="min-w-0 flex-1 truncate text-center font-mono text-[10px] uppercase tracking-[0.2em]">
          <span v-if="blocked" class="text-amber-400">{{ $t('onboarding.account.required') }}</span>
          <span v-else class="text-fg-faint">{{ $t(`onboarding.${step.id}.name`) }}</span>
        </p>

        <AppButton
            class="h-10 px-6 tracking-[0.2em]"
            :icon="isLast ? 'i-lucide-rocket' : 'i-lucide-arrow-right'"
            :loading="finishing"
            :disabled="blocked || finishing"
            @click="next"
        >
          {{ nextLabel }}
        </AppButton>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.step-forward-enter-active,
.step-back-enter-active {
  transition: opacity 0.35s cubic-bezier(0.16, 1, 0.3, 1),
              transform 0.35s cubic-bezier(0.16, 1, 0.3, 1),
              filter 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}

.step-forward-leave-active,
.step-back-leave-active {
  transition: opacity 0.18s ease-in, transform 0.18s ease-in, filter 0.18s ease-in;
}

.step-forward-enter-from {
  opacity: 0;
  transform: translateX(36px);
  filter: blur(6px);
}

.step-forward-leave-to {
  opacity: 0;
  transform: translateX(-24px);
  filter: blur(4px);
}

.step-back-enter-from {
  opacity: 0;
  transform: translateX(-36px);
  filter: blur(6px);
}

.step-back-leave-to {
  opacity: 0;
  transform: translateX(24px);
  filter: blur(4px);
}
</style>
