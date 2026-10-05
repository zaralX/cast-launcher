<script setup lang="ts">
const store = useAppStore()
const locales = useAvailableLocales()

const language = computed({
  get: () => store.config?.launcher.language ?? 'ru',
  set: (value: string) => {
    if (store.config) store.config.launcher.language = value
  },
})

const accent = computed({
  get: () => store.config?.launcher.accent ?? 'sky',
  set: (value: string) => {
    if (store.config) store.config.launcher.accent = value
  },
})

const compact = computed({
  get: () => store.config?.launcher.compact ?? false,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.compact = value
  },
})
</script>

<template>
  <OnboardingPane :title="$t('onboarding.appearance.title')">
    <div class="grid gap-10 lg:grid-cols-[minmax(0,1fr)_20rem]">
      <div class="space-y-8">
        <SettingsField :label="$t('settings.launcher.language')">
          <div class="flex flex-wrap gap-2">
            <button
              v-for="locale in locales"
              :key="locale.code"
              type="button"
              class="flex cursor-pointer items-center gap-2.5 border px-3.5 py-2 transition-colors duration-300"
              :class="language === locale.code
                ? 'border-fg text-fg'
                : 'border-line text-fg-muted hover:border-line-strong hover:text-fg'"
              @click="language = locale.code"
            >
              <UIcon
                v-if="flagOf(locale.code)"
                :name="flagOf(locale.code)!"
                mode="svg"
                class="size-4 shrink-0"
              />
              <span class="text-[13px]">{{ locale.name }}</span>
            </button>
          </div>
        </SettingsField>

        <SettingsField :label="$t('settings.launcher.accent.label')">
          <div class="flex flex-wrap gap-2">
            <button
              v-for="item in ACCENTS"
              :key="item.value"
              type="button"
              :title="$t(item.labelKey)"
              :aria-label="$t(item.labelKey)"
              :aria-pressed="accent === item.value"
              class="group grid size-9 cursor-pointer place-items-center border transition-colors duration-300"
              :class="accent === item.value ? 'border-fg' : 'border-line hover:border-line-strong'"
              @click="accent = item.value"
            >
              <span
                class="size-4 transition-transform duration-300 ease-deck group-hover:scale-110"
                :style="{ backgroundColor: item.preview }"
              />
            </button>
          </div>
        </SettingsField>

        <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ $t('settings.launcher.compact.label') }}
            </p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              {{ $t('settings.launcher.compact.hint') }}
            </p>
          </div>
          <USwitch
            v-model="compact"
            size="lg"
          />
        </div>
      </div>

      <aside class="space-y-3">
        <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
          {{ $t('onboarding.appearance.preview') }}
        </p>
        <OnboardingPreview :compact="compact" />
      </aside>
    </div>
  </OnboardingPane>
</template>
