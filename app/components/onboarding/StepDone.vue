<script setup lang="ts">
import {storeToRefs} from "pinia"
import {ACCENTS} from "~/composables/useAppearance"
import {useAccountStore} from "~/stores/account"
import {useAppStore} from "~/stores/app"
import type {ImportReport} from "~/types/import"

const props = defineProps<{
  report: ImportReport | null
}>()

const {t} = useI18n()
const store = useAppStore()
const accountStore = useAccountStore()
const locales = useAvailableLocales()
const {accountConfig} = storeToRefs(accountStore)

const launcher = computed(() => store.config?.launcher ?? null)

const accent = computed(() => ACCENTS.find(item => item.value === launcher.value?.accent) ?? ACCENTS[0]!)

const account = computed(() => {
  const config = accountConfig.value
  if (!config?.accounts?.length) return null
  return config.accounts[config.selected ?? 0] ?? null
})

const languageName = computed(() =>
    locales.value.find(locale => locale.code === launcher.value?.language)?.name
    ?? launcher.value?.language
    ?? "")

const onOff = (value: boolean | undefined) => value ? t("onboarding.done.on") : t("onboarding.done.off")

const rows = computed(() => [
  {
    icon: "i-lucide-languages",
    label: t("settings.launcher.language"),
    value: languageName.value,
    accented: false
  },
  {
    icon: "i-lucide-palette",
    label: t("settings.launcher.accent.label"),
    value: t(accent.value.labelKey),
    swatch: accent.value.preview,
    accented: false
  },
  {
    icon: "i-lucide-rows-3",
    label: t("settings.launcher.compact.label"),
    value: onOff(launcher.value?.compact),
    accented: launcher.value?.compact === true
  },
  {
    icon: "i-lucide-shield-check",
    label: t("settings.launcher.telemetry.label"),
    value: onOff(launcher.value?.telemetry),
    accented: launcher.value?.telemetry === true
  },
  {
    icon: "i-lucide-refresh-cw",
    label: t("settings.launcher.auto_update.label"),
    value: onOff(launcher.value?.auto_update),
    accented: launcher.value?.auto_update === true
  },
  {
    icon: "i-lucide-user-round",
    label: t("settings.accounts.title"),
    value: account.value?.name ?? t("settings.accounts.empty"),
    accented: !!account.value
  },
  {
    icon: "i-lucide-import",
    label: t("settings.import.title"),
    value: props.report
        ? t("onboarding.done.imported", {count: props.report.imported.length})
        : t("onboarding.done.not_imported"),
    accented: !!props.report?.imported.length
  }
])
</script>

<template>
  <OnboardingPane
      index="06 / 06"
      :title="$t('onboarding.done.title')"
      :subtitle="$t('onboarding.done.subtitle')"
  >
    <div class="grid gap-8 lg:grid-cols-[minmax(0,1fr)_15rem]">
      <ul class="grid gap-px border border-line bg-line sm:grid-cols-2">
        <li
            v-for="(row, i) in rows"
            :key="row.label"
            class="animate-rise flex items-center gap-4 bg-ink-800 px-5 py-4"
            :style="{ animationDelay: `${i * 60}ms` }"
        >
          <UIcon
              :name="row.icon"
              class="size-4 shrink-0 transition-colors duration-500"
              :class="row.accented ? 'text-acid' : 'text-fg-faint'"
          />

          <div class="min-w-0 flex-1">
            <p class="font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">{{ row.label }}</p>
            <p class="mt-1.5 flex items-center gap-2 truncate text-[13px] text-fg">
              <span v-if="row.swatch" class="size-3 shrink-0" :style="{ backgroundColor: row.swatch }"/>
              {{ row.value }}
            </p>
          </div>
        </li>
      </ul>

      <aside class="flex flex-col items-center justify-center border border-line bg-ink-800/40 px-6 py-10">
        <div class="relative grid size-24 place-items-center">
          <span class="absolute inset-0 border border-acid/30 animate-breathe"/>

          <svg viewBox="0 0 48 48" class="size-12" fill="none" aria-hidden="true">
            <path
                d="M10 25.5 L20 35 L38 14"
                stroke="currentColor"
                class="animate-tick text-acid"
                stroke-width="3"
                stroke-linecap="square"
                stroke-dasharray="48"
            />
          </svg>
        </div>

        <p class="mt-6 text-center font-unbounded text-[13px] font-semibold uppercase tracking-[0.16em] text-fg">
          {{ $t('onboarding.done.badge') }}
        </p>

        <p class="mt-3 text-center font-mono text-[10px] leading-relaxed text-fg-faint/70">
          {{ $t('onboarding.done.hint') }}
        </p>
      </aside>
    </div>
  </OnboardingPane>
</template>
