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
  info: "text-sky-400"
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
    <UChip :show="unseenCount > 0" :text="unseenCount" size="lg" color="error">
      <UButton
          icon="i-lucide-triangle-alert"
          class="h-12 aspect-square justify-center rounded-none"
          variant="ghost"
          :color="unseenCount > 0 ? 'error' : 'neutral'"
      />
    </UChip>

    <template #body>
      <div v-if="!entries.length" class="py-8 text-center text-sm text-zinc-400">
        Ошибок не было.
      </div>

      <div v-else class="space-y-2">
        <div
            v-for="entry in entries"
            :key="entry.id"
            class="rounded-lg border border-zinc-800 bg-zinc-900/50 p-3"
        >
          <div class="flex items-start gap-2">
            <Icon :name="entry.icon" :class="severityClass[entry.severity]" class="mt-0.5 shrink-0 text-lg"/>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="truncate font-medium">{{ entry.title }}</p>
                <UBadge v-if="entry.count > 1" size="sm" color="neutral" variant="subtle">
                  ×{{ entry.count }}
                </UBadge>
                <span class="ml-auto shrink-0 text-xs text-zinc-500">{{ formatTime(entry.at) }}</span>
              </div>

              <p v-if="entry.hint" class="mt-1 text-sm text-zinc-300">{{ entry.hint }}</p>

              <p v-if="entry.context.instanceName" class="mt-1 text-xs text-zinc-500">
                Сборка: {{ entry.context.instanceName }}
              </p>

              <details class="mt-2">
                <summary class="cursor-pointer select-none text-xs text-zinc-400 hover:text-zinc-200">
                  Технические детали
                </summary>
                <pre class="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-all rounded bg-zinc-950/70 p-2 text-xs text-zinc-400">{{ entry.report }}</pre>
              </details>
            </div>
          </div>

          <div class="mt-2 flex gap-2">
            <UButton
                size="xs"
                variant="ghost"
                color="neutral"
                :icon="copiedId === entry.id ? 'i-lucide-check' : 'i-lucide-copy'"
                @click="copy(entry.id, entry.report)"
            >
              {{ copiedId === entry.id ? 'Скопировано' : 'Копировать' }}
            </UButton>
            <UButton
                size="xs"
                variant="ghost"
                color="neutral"
                icon="i-lucide-x"
                @click="errorStore.dismiss(entry.id)"
            >
              Скрыть
            </UButton>
          </div>
        </div>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full gap-2">
        <UButton
            :disabled="!entries.length"
            variant="subtle"
            color="neutral"
            :icon="copiedId === 'all' ? 'i-lucide-check' : 'i-lucide-clipboard-list'"
            @click="copyAll"
        >
          Скопировать всё
        </UButton>
        <UButton
            :disabled="!entries.length"
            class="ml-auto"
            variant="subtle"
            color="error"
            icon="i-lucide-trash-2"
            @click="errorStore.clear()"
        >
          Очистить
        </UButton>
      </div>
    </template>
  </UModal>
</template>
