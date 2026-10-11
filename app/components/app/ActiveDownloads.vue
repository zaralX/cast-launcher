<script setup lang="ts">
const { t } = useI18n()
const instanceStore = useInstanceStore()
const { installs } = storeToRefs(instanceStore)

const open = ref(false)

const averageProgress = computed(() => {
  if (!installs.value.length) return 0
  const sum = installs.value.reduce((a, i) => a + i.progress, 0)
  return sum / installs.value.length
})

const percent = (value: number) => Math.round(value * 100)

const formatSize = (bytes: number) => {
  if (!bytes) return '-'
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} ${t('common.unit.kb')}`
  return `${(bytes / 1024 / 1024).toFixed(1)} ${t('common.unit.mb')}`
}

watch(installs, (value) => {
  if (!value.length) open.value = false
})
</script>

<template>
  <Dialog
    v-if="installs.length"
    v-model:open="open"
  >
    <DialogTrigger as-child>
      <button
        type="button"
        class="group relative flex max-w-md min-w-64 cursor-pointer items-center gap-2.5 overflow-hidden border border-line bg-control px-3 py-1 text-left outline-none transition-colors duration-300 hover:border-accent/50 hover:bg-hover focus-visible:border-accent/50"
      >
        <KitLiveDot
          size="sm"
          class="text-accent"
        />

        <span class="min-w-0 flex-1 truncate font-mono text-caption leading-none text-fg-muted">
          <template v-if="installs.length === 1">
            {{ installs[0]!.instanceName }} · {{ uiText(installs[0]!.phase) }}
          </template>
          <template v-else>
            {{ $t('download.installs', { count: installs.length }) }}
          </template>
        </span>

        <span class="shrink-0 font-mono text-caption leading-none tabular-nums text-accent">
          {{ percent(averageProgress) }}%
        </span>

        <Progress
          :model-value="percent(averageProgress)"
          class="absolute inset-x-0 bottom-0 bg-transparent"
        />
      </button>
    </DialogTrigger>

    <DialogContent>
      <DialogHeader>
        <DialogTitle>{{ $t('download.title') }}</DialogTitle>
      </DialogHeader>

      <DialogBody class="space-y-8">
        <section
          v-for="install in installs"
          :key="install.instanceId"
        >
          <header class="flex items-baseline justify-between gap-4">
            <div class="min-w-0">
              <h3 class="truncate font-unbounded text-lead font-semibold tracking-heading text-fg">
                {{ install.instanceName }}
              </h3>
              <p class="mt-1.5 truncate font-mono text-caption uppercase tracking-caps text-fg-faint">
                {{ uiText(install.phase) }}
              </p>
            </div>

            <span class="shrink-0 font-unbounded text-[22px] font-semibold leading-none tracking-[-0.05em] text-fg">
              {{ percent(install.progress) }}<span class="text-body text-accent">%</span>
            </span>
          </header>

          <Progress
            :model-value="percent(install.progress)"
            class="mt-3"
          />

          <ul
            v-if="install.files.length"
            class="mt-4 space-y-2.5"
          >
            <li
              v-for="file in install.files"
              :key="file.url"
              class="flex items-center gap-3"
            >
              <span class="min-w-0 flex-1 truncate font-mono text-caption text-fg-muted">{{ file.name }}</span>

              <span class="shrink-0 font-mono text-caption tabular-nums text-fg-faint">
                {{ formatSize(file.total) }}
              </span>

              <Progress
                :model-value="percent(file.percent)"
                tone="muted"
                class="w-20 shrink-0"
              />
            </li>
          </ul>

          <p
            v-else
            class="mt-4 truncate font-mono text-caption text-fg-faint"
          >
            {{ uiText(install.message) }}
          </p>

          <Button
            variant="danger-ghost"
            icon="i-lucide-circle-stop"
            class="mt-2"
            :loading="install.aborting"
            @click="instanceStore.abortInstall(install.instanceId)"
          >
            {{ install.aborting ? $t('download.aborting') : $t('download.abort') }}
          </Button>
        </section>
      </DialogBody>
    </DialogContent>
  </Dialog>
</template>
