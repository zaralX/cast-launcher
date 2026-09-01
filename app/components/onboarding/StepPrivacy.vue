<script setup lang="ts">
import {getVersion} from "@tauri-apps/api/app"
import {setTelemetryEnabled} from "~/composables/useTelemetry"
import {useAppStore} from "~/stores/app"

const store = useAppStore()

const telemetry = computed({
  get: () => store.config?.launcher.telemetry ?? true,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.telemetry = value
    setTelemetryEnabled(value)
  }
})

const autoUpdate = computed({
  get: () => store.config?.launcher.auto_update ?? true,
  set: (value: boolean) => {
    if (store.config) store.config.launcher.auto_update = value
  }
})

const SENT = ["version", "os", "crashes", "install_errors"]
const KEPT = ["nickname", "paths", "logs", "chats"]

const version = ref("")

onMounted(async () => {
  version.value = await getVersion().catch(() => "")
})

const nextVersion = computed(() => {
  const parts = version.value.split(".")
  if (parts.length !== 3) return ""

  const patch = Number(parts[2])
  return Number.isFinite(patch) ? `${parts[0]}.${parts[1]}.${patch + 1}` : ""
})
</script>

<template>
  <OnboardingPane
      index="03 / 06"
      :title="$t('onboarding.privacy.title')"
  >
    <div class="space-y-4">
      <OnboardingToggle
          v-model="telemetry"
          icon="i-lucide-shield-check"
          :label="$t('settings.launcher.telemetry.label')"
          :hint="$t('settings.launcher.telemetry.hint')"
      >
        <template #default="{ on }">
          <div class="grid gap-6 sm:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] sm:items-center">
            <div>
              <p class="font-mono text-[9px] uppercase tracking-[0.24em]" :class="on ? 'text-acid' : 'text-fg-faint'">
                {{ $t('onboarding.privacy.sent') }}
              </p>
              <ul class="mt-3 space-y-1.5">
                <li
                    v-for="item in SENT"
                    :key="item"
                    class="flex items-center gap-2 font-mono text-[10px] text-fg-muted transition-opacity duration-500"
                    :class="on ? 'opacity-100' : 'opacity-35'"
                >
                  <UIcon name="i-lucide-arrow-up-right" class="size-3 shrink-0" :class="on ? 'text-acid' : 'text-fg-faint'"/>
                  {{ $t(`onboarding.privacy.sent_items.${item}`) }}
                </li>
              </ul>
            </div>

            <div class="relative hidden h-16 w-24 items-center sm:flex">
              <span class="absolute inset-x-0 top-1/2 h-px -translate-y-1/2 bg-line"/>

              <template v-if="on">
                <span
                    v-for="i in 3"
                    :key="i"
                    class="animate-stream absolute top-1/2 size-1 -translate-y-1/2 bg-acid"
                    :style="{ animationDelay: `${(i - 1) * 0.85}s` }"
                />
              </template>

              <UIcon
                  name="i-lucide-server"
                  class="absolute right-0 top-1/2 size-4 -translate-y-1/2 bg-ink-800 transition-colors duration-500"
                  :class="on ? 'text-acid' : 'text-fg-faint/50'"
              />
            </div>

            <div>
              <p class="font-mono text-[9px] uppercase tracking-[0.24em] text-fg-faint">
                {{ $t('onboarding.privacy.never') }}
              </p>
              <ul class="mt-3 space-y-1.5">
                <li
                    v-for="item in KEPT"
                    :key="item"
                    class="flex items-center gap-2 font-mono text-[10px] text-fg-muted"
                >
                  <UIcon name="i-lucide-x" class="size-3 shrink-0 text-fg-faint"/>
                  {{ $t(`onboarding.privacy.never_items.${item}`) }}
                </li>
              </ul>
            </div>
          </div>
        </template>
      </OnboardingToggle>

      <OnboardingToggle
          v-model="autoUpdate"
          icon="i-lucide-refresh-cw"
          :label="$t('settings.launcher.auto_update.label')"
          :hint="$t('settings.launcher.auto_update.hint')"
      >
        <template #default="{ on }">
          <div class="flex flex-wrap items-center gap-4">
            <span
                class="border px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] transition-colors duration-500"
                :class="on ? 'border-line text-fg-faint' : 'border-acid/50 text-fg'"
            >
              v{{ version || '—' }}
            </span>

            <span class="relative h-px w-16 bg-line">
              <span
                  v-if="on"
                  class="animate-stream absolute top-1/2 size-1 -translate-y-1/2 bg-acid"
              />
            </span>

            <span
                class="border px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] transition-all duration-500 ease-deck"
                :class="on
                  ? 'border-acid bg-acid/10 text-acid'
                  : 'border-line text-fg-faint/40 line-through'"
            >
              v{{ nextVersion || '—' }}
            </span>
          </div>
        </template>
      </OnboardingToggle>
    </div>
  </OnboardingPane>
</template>
