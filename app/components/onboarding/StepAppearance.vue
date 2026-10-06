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
    <div class="grid grid-cols-[minmax(0,1fr)_20rem] gap-10">
      <div class="space-y-8">
        <KitField :label="$t('settings.launcher.language')">
          <RadioGroup
            v-model="language"
            variant="tiles"
          >
            <RadioGroupItem
              v-for="locale in locales"
              :key="locale.code"
              :value="locale.code"
              class="gap-2.5 px-3.5 py-2"
            >
              <Icon
                v-if="flagOf(locale.code)"
                :name="flagOf(locale.code)!"
                mode="svg"
                class="size-4 shrink-0"
              />
              <span class="text-title">{{ locale.name }}</span>
            </RadioGroupItem>
          </RadioGroup>
        </KitField>

        <KitField :label="$t('settings.launcher.accent.label')">
          <KitAccentPicker v-model="accent" />
        </KitField>

        <KitRow
          :label="$t('settings.launcher.compact.label')"
          :description="$t('settings.launcher.compact.hint')"
        >
          <Switch
            v-model="compact"
            size="lg"
          />
        </KitRow>
      </div>

      <aside class="space-y-3">
        <Label>{{ $t('onboarding.appearance.preview') }}</Label>
        <OnboardingPreview :compact="compact" />
      </aside>
    </div>
  </OnboardingPane>
</template>
