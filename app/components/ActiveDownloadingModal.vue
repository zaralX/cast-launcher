<script setup lang="ts">
const instanceStore = useInstanceStore()
const {installs} = storeToRefs(instanceStore)

const open = ref(false)

const averageProgress = computed(() => {
  if (!installs.value.length) return 0
  const sum = installs.value.reduce((a, i) => a + i.progress, 0)
  return sum / installs.value.length
})

const percent = (value: number) => Math.round(value * 100)

const formatSize = (bytes: number) => {
  if (!bytes) return "-"
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} КБ`
  return `${(bytes / 1024 / 1024).toFixed(1)} МБ`
}

watch(installs, (value) => {
  if (!value.length) open.value = false
})
</script>

<template>
  <UModal v-if="installs.length" v-model:open="open" title="Загрузка файлов" class="ml-auto mr-0">
    <button
        type="button"
        class="group relative ml-0! flex min-w-64 max-w-md items-center gap-2.5 overflow-hidden border border-line bg-ink-700 px-3 py-1 text-left transition-colors duration-300 hover:border-acid/50 hover:bg-ink-600"
    >
      <span class="relative grid size-1.5 shrink-0 place-items-center">
        <span class="absolute size-1.5 bg-acid animate-breathe"/>
        <span class="size-1.5 bg-acid"/>
      </span>

      <template v-if="installs.length === 1">
        <span class="min-w-0 flex-1 truncate font-mono text-[11px] leading-none text-fg-muted">
          {{ installs[0]!.instanceName }} · {{ installs[0]!.phase }}
        </span>
      </template>
      <template v-else>
        <span class="min-w-0 flex-1 truncate font-mono text-[11px] leading-none text-fg-muted">
          Установок: {{ installs.length }}
        </span>
      </template>

      <span class="shrink-0 font-mono text-[11px] leading-none tabular-nums text-acid">
        {{ percent(averageProgress) }}%
      </span>

      <span
          class="absolute bottom-0 left-0 h-px bg-acid transition-[width] duration-500 ease-deck"
          :style="{ width: `${percent(averageProgress)}%` }"
      />
    </button>

    <template #body>
      <div class="space-y-8">
        <section v-for="install in installs" :key="install.instanceId">
          <header class="flex items-baseline justify-between gap-4">
            <div class="min-w-0">
              <h3 class="truncate font-unbounded text-[14px] font-semibold tracking-[-0.035em] text-fg">
                {{ install.instanceName }}
              </h3>
              <p class="mt-1.5 flex items-center gap-2 truncate font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">
                {{ install.phase }}
              </p>
            </div>

            <span class="shrink-0 font-unbounded text-[22px] font-semibold leading-none tracking-[-0.05em] text-fg">
              {{ percent(install.progress) }}<span class="text-[12px] text-acid">%</span>
            </span>
          </header>

          <div class="mt-3 h-px w-full bg-line">
            <div
                class="h-px bg-acid transition-[width] duration-500 ease-deck"
                :style="{ width: `${percent(install.progress)}%` }"
            />
          </div>

          <!-- Активные файлы этой установки: строка на файл, обновляется на месте -->
          <ul v-if="install.files.length" class="mt-4 space-y-2.5">
            <li v-for="file in install.files" :key="file.url" class="flex items-center gap-3">
              <span class="min-w-0 flex-1 truncate font-mono text-[10px] text-fg-muted">{{ file.name }}</span>

              <span class="shrink-0 font-mono text-[10px] tabular-nums text-fg-faint">
                {{ formatSize(file.total) }}
              </span>

              <span class="relative block h-px w-20 shrink-0 bg-line">
                <span
                    class="absolute inset-y-0 left-0 bg-fg-muted transition-[width] duration-300 ease-deck"
                    :style="{ width: `${percent(file.percent)}%` }"
                />
              </span>
            </li>
          </ul>

          <p v-else class="mt-4 truncate font-mono text-[10px] text-fg-faint">{{ install.message }}</p>

          <button
              type="button"
              :disabled="install.aborting"
              class="mt-4 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.16em] text-fg-faint transition-colors duration-300 hover:text-red-400 disabled:pointer-events-none disabled:opacity-40"
              @click="instanceStore.abortInstall(install.instanceId)"
          >
            <UIcon
                :name="install.aborting ? 'i-lucide-loader-circle' : 'i-lucide-circle-stop'"
                class="size-3"
                :class="install.aborting ? 'animate-spin' : ''"
            />
            {{ install.aborting ? 'Останавливаем' : 'Прервать' }}
          </button>
        </section>
      </div>
    </template>
  </UModal>
</template>
