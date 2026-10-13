<script setup lang="ts">
const appStore = useAppStore()
const { update, updateProgress, updateBusy } = storeToRefs(appStore)

const { t, locale } = useI18n()

const open = ref(false)

const downloading = computed(() => !update.value?.downloaded && (updateBusy.value === 'downloading' || !!updateProgress.value))

const percent = computed(() => {
  const progress = updateProgress.value
  if (!progress?.total) return null
  return Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
})

const label = computed(() => {
  const version = update.value?.version ?? ''
  if (update.value?.downloaded) return t('update.ready', { version })
  if (downloading.value) return t('update.downloading', { version })
  return t('update.available', { version })
})

const date = computed(() => {
  const value = update.value?.date
  return value ? new Date(value).toLocaleDateString(locale.value) : ''
})

const install = () => safeRun(() => appStore.installUpdate(), { context: { action: t('settings.about.update_action') } })

const download = () => safeRun(() => appStore.downloadUpdate(), { context: { action: t('settings.about.update_action') } })
</script>

<template>
  <Popover
    v-if="update"
    v-model:open="open"
  >
    <PopoverTrigger as-child>
      <Button
        variant="ghost"
        size="sm"
        class="my-auto mr-1.5"
      >
        <Icon
          name="i-lucide-circle-arrow-up"
          class="size-3.5 shrink-0 text-accent"
        />
        {{ label }}
        <span
          v-if="downloading && percent != null"
          class="text-fg-faint tabular-nums"
        >{{ percent }}%</span>
      </Button>
    </PopoverTrigger>

    <PopoverContent
      align="end"
      class="w-90 overflow-hidden"
    >
      <div class="px-4 pt-3.5 pb-3">
        <p class="text-title font-semibold text-fg">
          Cast Launcher {{ update.version }}
        </p>
        <p
          v-if="date"
          class="mt-0.5 text-caption text-fg-faint"
        >
          {{ date }}
        </p>
      </div>

      <p
        v-if="update.notes"
        class="max-h-56 overflow-y-auto border-t border-line px-4 py-3 text-body whitespace-pre-line text-fg-muted"
      >
        {{ update.notes }}
      </p>

      <div class="border-t border-line px-4 py-3">
        <template v-if="downloading">
          <div class="flex items-center justify-between text-caption">
            <span class="text-fg-muted">{{ $t('update.downloading_label') }}</span>
            <span
              v-if="percent != null"
              class="text-fg tabular-nums"
            >{{ percent }}%</span>
          </div>
          <Progress
            :model-value="percent"
            class="mt-2"
          />
        </template>

        <div
          v-else
          class="flex items-center gap-2"
        >
          <span class="flex-1" />

          <Button
            v-if="!update.downloaded"
            variant="ghost"
            size="sm"
            icon="i-lucide-download"
            @click="download"
          >
            {{ $t('update.download') }}
          </Button>
          <Button
            variant="primary"
            size="sm"
            :icon="update.downloaded ? 'i-lucide-rotate-cw' : 'i-lucide-circle-arrow-up'"
            :loading="updateBusy === 'installing'"
            @click="install"
          >
            {{ update.downloaded ? $t('update.restart') : $t('update.install') }}
          </Button>
        </div>
      </div>
    </PopoverContent>
  </Popover>
</template>
