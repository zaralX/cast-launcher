<script setup lang="ts">
import {storeToRefs} from "pinia";
import {useAppStore} from "~/stores/app";
import {JAVA_SOURCE_LABELS, type AppConfig, type JavaRuntime} from "~/types/app";

const config = defineModel<AppConfig | null>()

const store = useAppStore()
const {javaRuntimes, javaScanning} = storeToRefs(store)

const gb = (mb?: number) => ((mb ?? 0) / 1024).toFixed(1).replace(".", ",")

const javaPath = computed({
  get: () => config.value?.java.java_path ?? "",
  set: (value: string) => {
    if (config.value) config.value.java.java_path = value
  }
})

const isAuto = computed(() => !javaPath.value.trim())
const isDetected = computed(() => javaRuntimes.value.some(r => r.path === javaPath.value))

const describe = (runtime: JavaRuntime) => [
  runtime.vendor,
  runtime.is_64bit ? "64-бит" : "32-бит",
  JAVA_SOURCE_LABELS[runtime.source] ?? runtime.source
].join(" · ")

const select = (runtime: JavaRuntime | null) => {
  javaPath.value = runtime?.path ?? ""
}

const rescan = () => safeRun(() => store.scanJava(true))

const manual = ref<JavaRuntime | null>(null)
const manualChecking = ref(false)
let probeId = 0
let probeTimer: ReturnType<typeof setTimeout> | null = null

watch([javaPath, javaRuntimes], () => {
  if (probeTimer) clearTimeout(probeTimer)

  const value = javaPath.value.trim()
  if (!value || isDetected.value) {
    manual.value = null
    manualChecking.value = false
    return
  }

  manualChecking.value = true
  const id = ++probeId

  probeTimer = setTimeout(async () => {
    let result: JavaRuntime | null = null
    try {
      result = await store.probeJava(value)
    } catch {}

    if (id !== probeId) return
    manual.value = result
    manualChecking.value = false
  }, 400)
}, {immediate: true})

onMounted(() => safeRun(() => store.scanJava()))
onBeforeUnmount(() => {
  if (probeTimer) clearTimeout(probeTimer)
})
</script>

<template>
  <SettingsPanel
      index="03"
      title="Java"
      description="Параметры виртуальной машины, с которыми стартует игра."
      icon="i-lucide-cpu"
  >
    <div class="space-y-7">
      <div class="grid gap-6 sm:grid-cols-2">
        <SettingsField label="Минимум RAM" :hint="`≈ ${gb(config!.java.min_ram)} ГБ`">
          <UInput
              v-model="config!.java.min_ram"
              type="number"
              :min="1"
              class="w-full"
              :ui="{ base: 'font-mono tabular-nums' }"
          >
            <template #trailing>
              <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">MB</span>
            </template>
          </UInput>
        </SettingsField>

        <SettingsField label="Максимум RAM" :hint="`≈ ${gb(config!.java.max_ram)} ГБ`">
          <UInput
              v-model="config!.java.max_ram"
              type="number"
              :min="config!.java.min_ram ?? 1"
              class="w-full"
              :ui="{ base: 'font-mono tabular-nums' }"
          >
            <template #trailing>
              <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">MB</span>
            </template>
          </UInput>
        </SettingsField>
      </div>

      <div>
        <div class="mb-2 flex items-center justify-between gap-4">
          <span class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Среда выполнения</span>

          <button
              type="button"
              :disabled="javaScanning"
              class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted transition-colors duration-300 hover:text-acid disabled:pointer-events-none disabled:opacity-40"
              @click="rescan"
          >
            <UIcon
                :name="javaScanning ? 'i-lucide-loader-circle' : 'i-lucide-refresh-cw'"
                class="size-3.5"
                :class="javaScanning ? 'animate-spin' : ''"
            />
            {{ javaScanning ? 'Поиск' : 'Обновить' }}
          </button>
        </div>

        <ul class="border-t border-line">
          <li
              class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 pr-1 transition-colors duration-300 hover:bg-ink-700"
              @click="select(null)"
          >
            <span
                class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
                :class="isAuto ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
            />

            <UIcon name="i-lucide-wand-sparkles" class="size-4 shrink-0 text-fg-faint"/>

            <div class="min-w-0 flex-1">
              <p class="truncate text-[13px] text-fg">Автоматически</p>
              <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
                <template v-if="store.autoJavaRuntime">
                  Java {{ store.autoJavaRuntime.major }} · {{ store.autoJavaRuntime.vendor }}
                </template>
                <template v-else-if="javaScanning">Идёт поиск</template>
                <template v-else>Java в системе не найдена</template>
              </p>
            </div>

            <span v-if="isAuto" class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-acid">
              Активна
            </span>
          </li>

          <li
              v-for="runtime in javaRuntimes"
              :key="runtime.path"
              class="group relative flex cursor-pointer items-center gap-4 border-b border-line py-3.5 pl-4 pr-1 transition-colors duration-300 hover:bg-ink-700"
              @click="select(runtime)"
          >
            <span
                class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
                :class="javaPath === runtime.path ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
            />

            <span class="shrink-0 font-mono text-[15px] leading-none tabular-nums text-fg-muted">
              {{ runtime.major }}
            </span>

            <div class="min-w-0 flex-1">
              <p class="truncate text-[13px] text-fg">
                Java {{ runtime.version }}
                <span v-if="!runtime.is_64bit" class="ml-2 font-mono text-[9px] uppercase tracking-[0.2em] text-amber-400">
                  32 бита
                </span>
              </p>
              <p class="mt-1 truncate font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
                {{ describe(runtime) }}
              </p>
              <p class="mt-1 truncate font-mono text-[10px] text-fg-faint/70">{{ runtime.path }}</p>
            </div>

            <span
                v-if="javaPath === runtime.path"
                class="shrink-0 font-mono text-[9px] uppercase tracking-[0.2em] text-acid"
            >
              Активна
            </span>
          </li>
        </ul>

        <p
            v-if="!javaRuntimes.length && !javaScanning"
            class="border-b border-line py-6 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
        >
          Ни одной Java не найдено
        </p>
      </div>

      <SettingsField
          label="Свой путь к Java"
          hint="Заполняется при выборе из списка. Можно указать вручную — файл java/javaw или папку JDK."
      >
        <UInput
            v-model="javaPath"
            placeholder="Автоматически"
            class="w-full"
            :ui="{ base: 'font-mono text-[12px]' }"
        />

        <p v-if="!isAuto && !isDetected" class="mt-2 flex items-center gap-2 font-mono text-[10px] tracking-[0.02em]">
          <template v-if="manualChecking">
            <UIcon name="i-lucide-loader-circle" class="size-3 shrink-0 animate-spin text-fg-faint"/>
            <span class="text-fg-faint">Проверяем путь</span>
          </template>
          <template v-else-if="manual">
            <UIcon name="i-lucide-check" class="size-3 shrink-0 text-acid"/>
            <span class="text-fg-muted">Java {{ manual.version }} · {{ manual.vendor }} · {{ manual.arch }}</span>
          </template>
          <template v-else>
            <UIcon name="i-lucide-triangle-alert" class="size-3 shrink-0 text-amber-400"/>
            <span class="text-amber-400">По этому пути рабочая Java не найдена</span>
          </template>
        </p>
      </SettingsField>
    </div>
  </SettingsPanel>
</template>
