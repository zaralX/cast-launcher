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

const SEVERITY: Record<ErrorSeverity, { icon: string, rule: string }> = {
  error: { icon: 'text-danger', rule: 'bg-danger/70' },
  warning: { icon: 'text-warning', rule: 'bg-warning/70' },
  info: { icon: 'text-acid', rule: 'bg-acid/70' },
}

function formatTime(at: number) {
  return new Date(at).toLocaleTimeString(locale.value)
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
  <Dialog v-model:open="open">
    <DialogTrigger as-child>
      <Button
        variant="ghost"
        size="icon-lg"
        :aria-label="$t('error.center.title')"
        class="group"
        :class="unseenCount > 0 && 'text-danger hover:text-danger'"
      >
        <Icon
          name="i-lucide-triangle-alert"
          class="size-4 transition-transform duration-500 ease-deck group-hover:-translate-y-0.5"
        />
      </Button>
    </DialogTrigger>

    <DialogContent>
      <DialogHeader>
        <DialogTitle>{{ $t('error.center.title') }}</DialogTitle>
      </DialogHeader>

      <DialogBody>
        <KitStatus
          v-if="!entries.length"
          :blink="false"
          class="justify-center py-12"
        >
          {{ $t('error.center.empty') }}
        </KitStatus>

        <div
          v-else
          class="space-y-3"
        >
          <article
            v-for="entry in entries"
            :key="entry.id"
            class="group relative border border-line bg-ink-900 p-4 pl-5 transition-colors duration-300 hover:border-line-strong"
          >
            <span
              class="absolute inset-y-0 left-0 w-[2px]"
              :class="SEVERITY[entry.severity].rule"
            />

            <div class="flex items-start gap-3">
              <Icon
                :name="entry.icon"
                class="mt-0.5 size-4 shrink-0"
                :class="SEVERITY[entry.severity].icon"
              />

              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <p class="truncate text-title font-medium text-fg">
                    {{ entry.title }}
                  </p>
                  <Badge
                    v-if="entry.count > 1"
                    class="normal-case tracking-normal"
                  >
                    ×{{ entry.count }}
                  </Badge>
                  <span class="ml-auto shrink-0 font-mono text-label tabular-nums text-fg-faint">
                    {{ formatTime(entry.at) }}
                  </span>
                </div>

                <p
                  v-if="entry.reason"
                  class="mt-2 text-body leading-relaxed text-fg-muted"
                >
                  {{ entry.reason }}
                </p>

                <p
                  v-if="entry.hint"
                  class="mt-1 text-body leading-relaxed text-fg-faint"
                >
                  {{ entry.hint }}
                </p>

                <p
                  v-if="entry.context.instanceName"
                  class="mt-2 font-mono text-label uppercase tracking-caps text-fg-faint"
                >
                  {{ $t('error.center.instance') }} · {{ entry.context.instanceName }}
                </p>

                <details class="mt-3">
                  <summary class="cursor-pointer font-mono text-label uppercase tracking-caps text-fg-faint transition-colors select-none hover:text-acid">
                    {{ $t('error.center.technical') }}
                  </summary>
                  <pre class="mt-3 max-h-48 overflow-auto border border-line bg-ink-800 p-3 font-mono text-caption leading-relaxed break-all whitespace-pre-wrap text-fg-muted">{{ entry.report }}</pre>
                </details>
              </div>
            </div>

            <div class="mt-3 flex gap-5 pl-7">
              <Button
                variant="quiet"
                :icon="copiedId === entry.id ? 'i-lucide-check' : 'i-lucide-copy'"
                @click="copy(entry.id, entry.report)"
              >
                {{ copiedId === entry.id ? $t('common.copied') : $t('common.copy') }}
              </Button>
              <Button
                variant="quiet"
                icon="i-lucide-x"
                @click="errorStore.dismiss(entry.id)"
              >
                {{ $t('error.center.hide') }}
              </Button>
            </div>
          </article>
        </div>
      </DialogBody>

      <DialogFooter class="justify-between">
        <Button
          variant="quiet"
          :icon="copiedId === 'all' ? 'i-lucide-check' : 'i-lucide-clipboard-list'"
          :disabled="!entries.length"
          @click="copyAll"
        >
          {{ $t('error.center.copy_all') }}
        </Button>

        <Button
          variant="quiet-danger"
          icon="i-lucide-trash-2"
          :disabled="!entries.length"
          @click="errorStore.clear()"
        >
          {{ $t('error.center.clear') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
