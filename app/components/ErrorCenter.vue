<script setup lang="ts">
import {useErrorStore} from "~/stores/error";

const errorStore = useErrorStore()
const {entries, unseenCount} = storeToRefs(errorStore)

const open = useErrorCenterOpen()
const copiedId = ref<string | null>(null)

watch(open, (isOpen) => {
  if (isOpen) errorStore.markAllSeen()
}, {immediate: true})

const severityClass: Record<string, string> = {
  error: "text-red-400",
  warning: "text-amber-400",
  info: "text-acid"
}

const severityRule: Record<string, string> = {
  error: "bg-red-400/70",
  warning: "bg-amber-400/70",
  info: "bg-acid/70"
}

function formatTime(at: number) {
  return new Date(at).toLocaleTimeString("ru-RU")
}

async function copy(id: string, text: string) {
  try {
    await navigator.clipboard.writeText(text)
    copiedId.value = id
    setTimeout(() => {
      if (copiedId.value === id) copiedId.value = null
    }, 2000)
  } catch (e) {
    captureError(e, {code: "UNKNOWN", context: {action: "clipboard"}, toast: false})
  }
}

function copyAll() {
  copy("all", entries.value.map(e => e.report).join("\n\n———\n\n"))
}
</script>

<template>
  <UModal v-model:open="open" title="Журнал ошибок">
    <UChip :show="unseenCount > 0" :text="unseenCount" size="lg" color="error" class="flex">
      <UButton
          color="neutral"
          variant="ghost"
          aria-label="Журнал ошибок"
          class="group h-11 w-11 justify-center hover:bg-ink-600"
          :class="unseenCount > 0 ? 'text-red-400' : 'text-fg-faint hover:text-fg'"
      >
        <UIcon
            name="i-lucide-triangle-alert"
            class="size-4 transition-transform duration-500 ease-deck group-hover:-translate-y-0.5"
        />
      </UButton>
    </UChip>

    <template #body>
      <div v-if="!entries.length" class="flex flex-col items-center gap-3 py-12">
        <span class="size-1.5 bg-fg-faint"/>
        <p class="font-mono text-[10px] uppercase tracking-[0.28em] text-fg-faint">Ошибок не было</p>
      </div>

      <div v-else class="space-y-3">
        <article
            v-for="entry in entries"
            :key="entry.id"
            class="group relative border border-line bg-ink-900 p-4 pl-5 transition-colors duration-300 hover:border-line-strong"
        >
          <span class="absolute inset-y-0 left-0 w-[2px]" :class="severityRule[entry.severity]"/>

          <div class="flex items-start gap-3">
            <UIcon :name="entry.icon" :class="severityClass[entry.severity]" class="mt-0.5 size-4 shrink-0"/>

            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="truncate text-[13px] font-medium text-fg">{{ entry.title }}</p>
                <span
                    v-if="entry.count > 1"
                    class="shrink-0 border border-line px-1.5 py-px font-mono text-[9px] text-fg-muted"
                >
                  ×{{ entry.count }}
                </span>
                <span class="ml-auto shrink-0 font-mono text-[10px] tabular-nums text-fg-faint">
                  {{ formatTime(entry.at) }}
                </span>
              </div>

              <p v-if="entry.hint" class="mt-2 text-[12px] leading-relaxed text-fg-muted">{{ entry.hint }}</p>

              <p
                  v-if="entry.context.instanceName"
                  class="mt-2 font-mono text-[10px] uppercase tracking-[0.16em] text-fg-faint"
              >
                Сборка · {{ entry.context.instanceName }}
              </p>

              <details class="mt-3">
                <summary
                    class="cursor-pointer select-none font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint transition-colors hover:text-acid"
                >
                  Технические детали
                </summary>
                <pre
                    class="mt-3 max-h-48 overflow-auto whitespace-pre-wrap break-all border border-line bg-ink-800 p-3 font-mono text-[11px] leading-relaxed text-fg-muted"
                >{{ entry.report }}</pre>
              </details>
            </div>
          </div>

          <div class="mt-3 flex gap-5 pl-7">
            <AppButton
                tone="quiet"
                class="text-[10px] tracking-[0.16em] text-fg-faint"
                :icon="copiedId === entry.id ? 'i-lucide-check' : 'i-lucide-copy'"
                @click="copy(entry.id, entry.report)"
            >
              {{ copiedId === entry.id ? 'Скопировано' : 'Копировать' }}
            </AppButton>
            <AppButton
                tone="quiet"
                class="text-[10px] tracking-[0.16em] text-fg-faint hover:text-fg"
                icon="i-lucide-x"
                @click="errorStore.dismiss(entry.id)"
            >
              Скрыть
            </AppButton>
          </div>
        </article>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full items-center gap-6">
        <AppButton
            tone="quiet"
            class="text-[10px] tracking-[0.18em]"
            :icon="copiedId === 'all' ? 'i-lucide-check' : 'i-lucide-clipboard-list'"
            :disabled="!entries.length"
            @click="copyAll"
        >
          Скопировать всё
        </AppButton>

        <AppButton
            tone="quiet"
            class="ml-auto text-[10px] tracking-[0.18em] hover:text-red-400"
            icon="i-lucide-trash-2"
            :disabled="!entries.length"
            @click="errorStore.clear()"
        >
          Очистить
        </AppButton>
      </div>
    </template>
  </UModal>
</template>
