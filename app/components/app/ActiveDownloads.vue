<script setup lang="ts">
const instanceStore = useInstanceStore()
const { installs } = storeToRefs(instanceStore)

const open = ref(false)

const averageProgress = computed(() => {
  if (!installs.value.length) return 0
  const sum = installs.value.reduce((a, i) => a + i.progress, 0)
  return sum / installs.value.length
})

const percent = (value: number) => Math.round(value * 100)

watch(installs, (value) => {
  if (!value.length) open.value = false
})
</script>

<template>
  <Popover
    v-if="installs.length"
    v-model:open="open"
  >
    <PopoverTrigger as-child>
      <Button
        variant="ghost"
        size="sm"
        class="my-auto mr-1.5 max-w-72"
      >
        <Icon
          name="i-lucide-download"
          class="size-3.5 shrink-0 text-accent"
        />
        <span class="min-w-0 truncate">
          <template v-if="installs.length === 1">{{ installs[0]!.instanceName }}</template>
          <template v-else>{{ $t('download.installs', { count: installs.length }) }}</template>
        </span>
        <Progress
          :model-value="percent(averageProgress)"
          class="w-10 shrink-0"
        />
        <span class="shrink-0 tabular-nums">{{ percent(averageProgress) }}%</span>
      </Button>
    </PopoverTrigger>

    <PopoverContent
      align="end"
      class="w-90 overflow-hidden"
    >
      <p class="px-4 pt-3.5 pb-2 text-title font-semibold text-fg">
        {{ $t('download.title') }}
      </p>

      <div class="max-h-[min(30rem,calc(100vh-6rem))] overflow-y-auto">
        <section
          v-for="install in installs"
          :key="install.instanceId"
          class="border-t border-line px-4 py-3"
        >
          <div class="flex items-baseline justify-between gap-3">
            <p class="min-w-0 truncate font-medium text-fg">
              {{ install.instanceName }}
            </p>
            <span class="shrink-0 text-caption text-fg tabular-nums">{{ percent(install.progress) }}%</span>
          </div>
          <p class="mt-0.5 truncate text-caption text-fg-muted">
            {{ uiText(install.phase) }}
          </p>

          <div class="mt-2 flex items-center gap-2.5">
            <Progress
              :model-value="percent(install.progress)"
              class="flex-1"
            />
            <Button
              variant="danger-ghost"
              size="sm"
              :loading="install.aborting"
              @click="instanceStore.abortInstall(install.instanceId)"
            >
              {{ install.aborting ? $t('download.aborting') : $t('download.abort') }}
            </Button>
          </div>

          <ul
            v-if="install.files.length"
            class="mt-2 space-y-1"
          >
            <li
              v-for="file in install.files"
              :key="file.url"
              class="flex items-center gap-2.5 text-micro text-fg-faint"
            >
              <span class="min-w-0 flex-1 truncate font-mono">{{ file.name }}</span>
              <span
                v-if="file.total"
                class="shrink-0 font-mono tabular-nums"
              >{{ formatBytes(file.total) }}</span>
              <Progress
                :model-value="percent(file.percent)"
                tone="muted"
                class="w-14 shrink-0"
              />
            </li>
          </ul>
          <p
            v-else-if="install.message"
            class="mt-2 truncate text-micro text-fg-faint"
          >
            {{ uiText(install.message) }}
          </p>
        </section>
      </div>
    </PopoverContent>
  </Popover>
</template>
