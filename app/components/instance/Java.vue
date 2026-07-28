<script setup lang="ts">
import {storeToRefs} from "pinia";
import {useAppStore} from "~/stores/app";
import type {InstanceSettings} from "~/types/instance";

const settings = defineModel<InstanceSettings>({required: true})

const {config} = storeToRefs(useAppStore())

const globalJava = computed(() => config.value?.java ?? null)

const gb = (mb?: number) => ((mb ?? 0) / 1024).toFixed(1).replace(".", ",")

const minRam = computed({
  get: () => settings.value.overrideMemory ? settings.value.minRam : globalJava.value?.min_ram ?? 0,
  set: (value: number) => settings.value.minRam = Number(value) || 0
})

const maxRam = computed({
  get: () => settings.value.overrideMemory ? settings.value.maxRam : globalJava.value?.max_ram ?? 0,
  set: (value: number) => settings.value.maxRam = Number(value) || 0
})

const javaMode = computed({
  get: () => settings.value.overrideJava ? settings.value.javaMode : globalJava.value?.java_mode ?? "auto",
  set: (value: typeof settings.value.javaMode) => settings.value.javaMode = value
})

const javaPath = computed({
  get: () => settings.value.overrideJava ? settings.value.javaPath : globalJava.value?.java_path ?? "",
  set: (value: string) => settings.value.javaPath = value
})

watch(() => settings.value.overrideMemory, (enabled) => {
  if (!enabled) return
  if (!settings.value.minRam) settings.value.minRam = globalJava.value?.min_ram ?? 1024
  if (!settings.value.maxRam) settings.value.maxRam = globalJava.value?.max_ram ?? 4096
})

watch(() => settings.value.overrideJava, (enabled) => {
  if (!enabled) return
  if (settings.value.javaMode === "auto" && !settings.value.javaPath.trim()) {
    settings.value.javaMode = globalJava.value?.java_mode ?? "auto"
    settings.value.javaPath = globalJava.value?.java_path ?? ""
  }
})
</script>

<template>
  <div class="space-y-6">
    <SettingsPanel
        index="03"
        title="Память"
        description="Сколько оперативной памяти получает игра этой сборки."
        icon="i-lucide-memory-stick"
    >
      <div class="space-y-7">
        <div class="flex items-center justify-between gap-6">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Свои значения</p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              Без переопределения используются значения из настроек лаунчера.
            </p>
          </div>
          <USwitch v-model="settings.overrideMemory" size="lg"/>
        </div>

        <div
            class="grid gap-6 border-t border-line pt-6 transition-opacity duration-300 sm:grid-cols-2"
            :class="settings.overrideMemory ? '' : 'opacity-45'"
        >
          <SettingsField label="Минимум RAM" :hint="`≈ ${gb(minRam)} ГБ`">
            <UInput
                v-model="minRam"
                type="number"
                :min="1"
                :disabled="!settings.overrideMemory"
                class="w-full"
                :ui="{ base: 'font-mono tabular-nums' }"
            >
              <template #trailing>
                <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">MB</span>
              </template>
            </UInput>
          </SettingsField>

          <SettingsField label="Максимум RAM" :hint="`≈ ${gb(maxRam)} ГБ`">
            <UInput
                v-model="maxRam"
                type="number"
                :min="minRam || 1"
                :disabled="!settings.overrideMemory"
                class="w-full"
                :ui="{ base: 'font-mono tabular-nums' }"
            >
              <template #trailing>
                <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">MB</span>
              </template>
            </UInput>
          </SettingsField>
        </div>
      </div>
    </SettingsPanel>

    <SettingsPanel
        index="04"
        title="Java"
        description="С какой Java запускать и устанавливать именно эту сборку."
        icon="i-lucide-cpu"
    >
      <div class="space-y-7">
        <div class="flex items-center justify-between gap-6">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Своя Java</p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              Полезно для старых версий и модпаков, которым нужна конкретная сборка Java.
            </p>
          </div>
          <USwitch v-model="settings.overrideJava" size="lg"/>
        </div>

        <div
            class="border-t border-line pt-6 transition-opacity duration-300"
            :class="settings.overrideJava ? '' : 'pointer-events-none opacity-45'"
        >
          <SettingsJavaRuntimes v-model:mode="javaMode" v-model:path="javaPath"/>
        </div>
      </div>
    </SettingsPanel>
  </div>
</template>
