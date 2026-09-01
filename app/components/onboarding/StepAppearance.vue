<script setup lang="ts">
import {ACCENTS} from "~/composables/useAppearance"
import {useAppStore} from "~/stores/app"

const store = useAppStore()

const accent = computed({
  get: () => store.config?.launcher.accent ?? "sky",
  set: (value: string) => {
    if (store.config) store.config.launcher.accent = value
  }
})

const compact = computed({
  get: () => store.config?.launcher.compact ?? false,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.compact = value
  }
})
</script>

<template>
  <OnboardingPane
      index="02 / 06"
      :title="$t('onboarding.appearance.title')"
  >
    <div class="grid gap-8 lg:grid-cols-[minmax(0,1fr)_18rem]">
      <div class="space-y-7">
        <div>
          <p class="mb-3 font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            {{ $t('settings.launcher.accent.label') }}
          </p>

          <div class="grid grid-cols-6 gap-2 sm:grid-cols-8 lg:grid-cols-6 xl:grid-cols-8">
            <button
                v-for="(item, i) in ACCENTS"
                :key="item.value"
                type="button"
                :title="$t(item.labelKey)"
                :aria-label="$t(item.labelKey)"
                :aria-pressed="accent === item.value"
                class="group animate-rise relative grid aspect-square cursor-pointer place-items-center border transition-colors duration-300"
                :class="accent === item.value ? 'border-fg' : 'border-line hover:border-line-strong'"
                :style="{ animationDelay: `${i * 35}ms` }"
                @click="accent = item.value"
            >
              <span
                  class="transition-all duration-500 ease-deck"
                  :class="accent === item.value ? 'size-7' : 'size-5 group-hover:size-6'"
                  :style="{ backgroundColor: item.preview }"
              />
              <span
                  v-if="accent === item.value"
                  class="animate-pop absolute -right-px -top-px size-1.5 bg-fg"
              />
            </button>
          </div>

          <Transition name="label" mode="out-in">
            <p :key="accent" class="mt-3 font-mono text-[10px] uppercase tracking-[0.24em] text-acid">
              {{ $t(ACCENTS.find(item => item.value === accent)?.labelKey ?? 'accent.sky') }}
            </p>
          </Transition>
        </div>

        <OnboardingToggle
            v-model="compact"
            icon="i-lucide-rows-3"
            :label="$t('settings.launcher.compact.label')"
            :hint="$t('settings.launcher.compact.hint')"
        />
      </div>

      <aside class="space-y-3">
        <div class="flex items-center gap-2.5">
          <UIcon name="i-lucide-monitor-play" class="size-3.5 text-acid"/>
          <p class="font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">
            {{ $t('onboarding.appearance.preview') }}
          </p>
        </div>

        <OnboardingPreview :compact="compact"/>
      </aside>
    </div>
  </OnboardingPane>
</template>

<style scoped>
.label-enter-active,
.label-leave-active {
  transition: opacity 0.2s ease, transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.label-enter-from {
  opacity: 0;
  transform: translateX(-6px);
}

.label-leave-to {
  opacity: 0;
  transform: translateX(6px);
}
</style>
