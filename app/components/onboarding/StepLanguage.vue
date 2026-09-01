<script setup lang="ts">
import {flagOf} from "~/composables/useLanguage"
import {useAppStore} from "~/stores/app"

const store = useAppStore()
const locales = useAvailableLocales()

const language = computed({
  get: () => store.config?.launcher.language ?? "ru",
  set: (value: string) => {
    if (store.config) store.config.launcher.language = value
  }
})
</script>

<template>
  <OnboardingPane
      index="01 / 06"
      :title="$t('onboarding.language.title')"
      :subtitle="$t('onboarding.language.subtitle')"
  >
    <div class="grid gap-8 lg:grid-cols-[minmax(0,1fr)_16rem]">
      <div class="grid gap-3 sm:grid-cols-2">
        <button
            v-for="(locale, i) in locales"
            :key="locale.code"
            type="button"
            class="group animate-rise relative flex cursor-pointer items-center gap-4 border px-5 py-4 text-left transition-colors duration-300"
            :class="language === locale.code
              ? 'border-acid/60 bg-ink-800'
              : 'border-line bg-ink-800/40 hover:border-line-strong hover:bg-ink-700'"
            :style="{ animationDelay: `${i * 70}ms` }"
            @click="language = locale.code"
        >
          <span
              class="absolute inset-y-0 left-0 w-[2px] origin-top bg-acid transition-transform duration-500 ease-deck"
              :class="language === locale.code ? 'scale-y-100' : 'scale-y-0'"
          />

          <span
              class="relative grid size-10 shrink-0 place-items-center overflow-hidden border transition-colors duration-500 ease-deck"
              :class="language === locale.code
                ? 'border-acid'
                : 'border-line group-hover:border-line-strong'"
          >
            <UIcon
                v-if="flagOf(locale.code)"
                :name="flagOf(locale.code)!"
                mode="svg"
                class="size-full transition-all duration-500 ease-deck"
                :class="language === locale.code
                  ? 'scale-100 opacity-100 saturate-100'
                  : 'scale-95 opacity-55 saturate-0 group-hover:scale-100 group-hover:opacity-90 group-hover:saturate-100'"
            />

            <span
                v-else
                class="font-mono text-[11px] uppercase tracking-[0.1em] transition-colors duration-500"
                :class="language === locale.code ? 'text-acid' : 'text-fg-faint group-hover:text-fg'"
            >
              {{ locale.code }}
            </span>
          </span>

          <span class="min-w-0 flex-1">
            <span class="block truncate text-[13px] text-fg">{{ locale.name }}</span>
            <span class="mt-1 block font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
              {{ locale.code }} ·
              {{ language === locale.code ? $t('onboarding.language.current') : $t('onboarding.language.switch') }}
            </span>
          </span>

          <UIcon
              v-if="language === locale.code"
              name="i-lucide-check"
              class="animate-pop size-4 shrink-0 text-acid"
          />
        </button>
      </div>
    </div>
  </OnboardingPane>
</template>

<style scoped>
.sample-enter-active,
.sample-leave-active {
  transition: opacity 0.25s ease, transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.sample-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

.sample-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
