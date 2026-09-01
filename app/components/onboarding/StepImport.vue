<script setup lang="ts">
import type {ImportReport} from "~/types/import"

const model = defineModel<ImportReport | null>({default: null})
</script>

<template>
  <OnboardingPane
      index="05 / 06"
      :title="$t('onboarding.import.title')"
      :subtitle="$t('onboarding.import.subtitle')"
  >
    <div class="space-y-6">
      <div class="relative flex items-start gap-4 overflow-hidden border border-line bg-ink-800/50 px-6 py-5">
        <span class="pointer-events-none absolute -right-14 -top-14 size-32 rounded-full bg-acid/[0.10] blur-3xl"/>

        <UIcon name="i-lucide-package-open" class="mt-0.5 size-5 shrink-0 text-acid"/>

        <div class="min-w-0 flex-1">
          <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            {{ $t('onboarding.import.optional') }}
          </p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
            {{ $t('onboarding.import.hint') }}
          </p>
        </div>

        <div class="hidden shrink-0 items-center gap-2 sm:flex">
          <span class="size-6 border border-line-strong"/>
          <span class="relative h-px w-10 bg-line">
            <span class="animate-stream absolute top-1/2 size-1 -translate-y-1/2 bg-acid"/>
          </span>
          <span class="size-6 border border-acid/60 bg-acid/10"/>
        </div>
      </div>

      <ImportWizard @imported="report => model = report"/>
    </div>
  </OnboardingPane>
</template>
