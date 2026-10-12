<script setup lang="ts">
import type { ErrorSeverity } from '~/types/error'

const { t, locale } = useI18n()
const toast = useAppToast()
const errorStore = useErrorStore()
const { entries, unseenCount } = storeToRefs(errorStore)

const open = useErrorCenterOpen()
const copiedId = ref<string | null>(null)

watch(open, (isOpen) => {
  if (isOpen) errorStore.markAllSeen()
}, { immediate: true })

const SEVERITY: Record<ErrorSeverity, string> = {
  error: 'bg-danger/16 text-danger',
  warning: 'bg-warning/16 text-warning',
  info: 'bg-accent/16 text-accent',
}

function formatTime(at: number) {
  return new Date(at).toLocaleTimeString(locale.value, { hour: '2-digit', minute: '2-digit' })
}

async function copy(id: string, text: string) {
  if (!await copyToClipboard(text)) {
    toast.add({ title: t('common.copy_failed'), color: 'error', icon: 'i-lucide-clipboard-x' })
    return
  }

  copiedId.value = id
  setTimeout(() => {
    if (copiedId.value === id) copiedId.value = null
  }, 2000)
}

function copyAll() {
  copy('all', entries.value.map(e => e.report).join('\n\n-------\n\n'))
}
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <Button
        variant="ghost"
        size="bar"
        :aria-label="$t('error.center.title')"
        :title="$t('error.center.title')"
      >
        <span class="relative">
          <Icon
            name="i-lucide-bell"
            class="size-4"
          />
          <span
            v-if="unseenCount > 0"
            class="absolute -top-0.5 -right-0.5 size-1.75 rounded-pill bg-danger ring-2 ring-frame"
          />
        </span>
      </Button>
    </PopoverTrigger>

    <PopoverContent
      align="end"
      :side-offset="6"
      class="flex max-h-[min(36rem,calc(100vh-4rem))] w-105 flex-col overflow-hidden"
    >
      <header class="flex shrink-0 items-center gap-1 pt-3 pr-2 pb-2.5 pl-4">
        <p class="flex-1 text-lead font-semibold text-fg">
          {{ $t('error.center.title') }}
        </p>
        <Button
          variant="ghost"
          size="icon-sm"
          :icon="copiedId === 'all' ? 'i-lucide-check' : 'i-lucide-clipboard-list'"
          :disabled="!entries.length"
          :aria-label="$t('error.center.copy_all')"
          :title="$t('error.center.copy_all')"
          @click="copyAll"
        />
        <Button
          variant="danger-ghost"
          size="icon-sm"
          icon="i-lucide-trash-2"
          :disabled="!entries.length"
          :aria-label="$t('error.center.clear')"
          :title="$t('error.center.clear')"
          @click="errorStore.clear()"
        />
      </header>

      <div
        v-if="!entries.length"
        class="flex flex-col items-center gap-2.5 border-t border-line px-4 py-10 text-center text-body text-fg-faint"
      >
        <Icon
          name="i-lucide-bell-off"
          class="size-5"
        />
        {{ $t('error.center.empty') }}
      </div>

      <div
        v-else
        class="min-h-0 flex-1 divide-y divide-line overflow-y-auto border-t border-line"
      >
        <article
          v-for="entry in entries"
          :key="entry.id"
          class="flex gap-3 py-3 pr-3.5 pl-4"
        >
          <span
            class="grid size-7.5 shrink-0 place-items-center rounded-sm"
            :class="SEVERITY[entry.severity]"
          >
            <Icon
              :name="entry.icon"
              class="size-4"
            />
          </span>

          <div class="min-w-0 flex-1">
            <div class="flex items-baseline gap-2.5">
              <p class="min-w-0 flex-1 font-medium text-fg">
                {{ entry.title }}
              </p>
              <Badge v-if="entry.count > 1">
                ×{{ entry.count }}
              </Badge>
              <span class="shrink-0 text-micro text-fg-faint tabular-nums">
                {{ formatTime(entry.at) }}
              </span>
            </div>

            <p
              v-if="entry.reason"
              class="mt-0.5 text-caption text-fg-muted"
            >
              {{ entry.reason }}
            </p>
            <p
              v-if="entry.hint"
              class="mt-0.5 text-caption text-fg-faint"
            >
              {{ entry.hint }}
            </p>
            <p
              v-if="entry.context.instanceName"
              class="mt-0.5 text-caption text-fg-faint"
            >
              {{ $t('error.center.instance') }} · {{ entry.context.instanceName }}
            </p>

            <details class="group/details mt-1.5">
              <summary class="flex cursor-pointer list-none items-center gap-1 text-caption text-fg-faint select-none hover:text-fg">
                <Icon
                  name="i-lucide-chevron-right"
                  class="size-3.5 transition-transform group-open/details:rotate-90"
                />
                {{ $t('error.center.technical') }}
              </summary>
              <pre class="mt-1.5 max-h-40 overflow-auto rounded-sm bg-sheet p-2.5 font-mono text-micro break-all whitespace-pre-wrap text-fg-muted">{{ entry.report }}</pre>
            </details>

            <div class="mt-1.5 -ml-2.5 flex gap-1">
              <Button
                variant="ghost"
                size="sm"
                :icon="copiedId === entry.id ? 'i-lucide-check' : 'i-lucide-copy'"
                @click="copy(entry.id, entry.report)"
              >
                {{ copiedId === entry.id ? $t('common.copied') : $t('common.copy') }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                icon="i-lucide-x"
                @click="errorStore.dismiss(entry.id)"
              >
                {{ $t('error.center.hide') }}
              </Button>
            </div>
          </div>
        </article>
      </div>
    </PopoverContent>
  </Popover>
</template>
