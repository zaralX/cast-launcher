<script setup lang="ts">
import {getCurrentWindow} from "@tauri-apps/api/window";
import type {BlockedFile} from "~/types/catalog";
import type {InstallSnapshot} from "~/types/instance";
import {call} from "~/types/backend";

const FOCUS_COOLDOWN = 2000

const props = defineProps<{ install: InstallSnapshot }>()

const files = ref<BlockedFile[]>(props.install.blocked ?? [])
const folder = ref("")
const scanning = ref(false)
const finishing = ref(false)

const missing = computed(() => files.value.filter(file => !file.localPath))
const found = computed(() => files.value.filter(file => file.localPath))
const allFound = computed(() => files.value.length > 0 && missing.value.length === 0)

let lastRescan = 0

watch(() => props.install.blocked, value => {
  if (value?.length) files.value = value
})

onMounted(async () => {
  folder.value = await safeRun(() => call("downloads_dir")) ?? ""

  const current = await safeRun(() => call("awaited_files", {instanceId: props.install.instanceId}))
  if (current?.length) files.value = current

  const unlisten = await safeRun(() => getCurrentWindow().onFocusChanged(({payload}) => {
    if (payload) rescan()
  }))

  if (unlisten) onScopeDispose(unlisten)
})

async function rescan() {
  if (scanning.value || allFound.value) return
  if (Date.now() - lastRescan < FOCUS_COOLDOWN) return

  lastRescan = Date.now()
  scanning.value = true

  try {
    const result = await call("rescan_files", {instanceId: props.install.instanceId})
    if (result?.length) files.value = result
  } catch {
    // z
  } finally {
    scanning.value = false
  }
}

async function scan() {
  if (!folder.value.trim() || scanning.value) return

  lastRescan = Date.now()
  scanning.value = true

  const result = await safeRun(
      () => call("scan_for_files", {instanceId: props.install.instanceId, folder: folder.value}),
      {context: {action: "Поиск скачанных файлов", instanceId: props.install.instanceId}}
  )

  if (result?.length) files.value = result

  scanning.value = false
}

async function pickFolder() {
  const picked = await safeRun(() => call("pick_folder", {title: "Папка со скачанными модами"}))

  if (!picked) return

  folder.value = picked
  await scan()
}

const openFile = (file: BlockedFile) => safeRun(() => call("open_url", {url: file.websiteUrl}))

async function openMissing() {
  for (const file of missing.value) {
    if (file.websiteUrl) await openFile(file)
  }
}

async function finish() {
  finishing.value = true
  await safeRun(() => call("resume_install", {instanceId: props.install.instanceId}))
  finishing.value = false
}

const cancel = () => safeRun(() => call("cancel_install", {instanceId: props.install.instanceId}))
</script>

<template>
  <UModal
      :open="true"
      :dismissible="false"
      :close="false"
      title="Нужно скачать вручную"
      :ui="{ content: 'max-w-2xl' }"
  >
    <template #body>
      <div class="space-y-6">
        <p class="text-[12px] leading-relaxed text-fg-muted">
          Авторы этих файлов запретили сторонним лаунчерам их раздавать - CurseForge не даёт на них прямых
          ссылок. Скачайте их со страниц ниже: лаунчер сам следит за папкой загрузок, сверяет файлы по
          контрольной сумме и отмечает их здесь. Переименовывать скачанное не нужно, но и не страшно.
        </p>

        <div>
          <label
              for="blocked-folder"
              class="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            <span>Где искать</span>
            <span v-if="!allFound" class="flex items-center gap-1.5 text-acid/70">
              <span class="size-1 animate-pulse rounded-full bg-acid"/>
              проверяется автоматически
            </span>
          </label>

          <div class="flex gap-2">
            <UInput
                id="blocked-folder"
                v-model="folder"
                placeholder="Папка загрузок"
                class="min-w-0 flex-1"
                @keydown.enter.prevent="scan"
            />
            <AppButton
                tone="quiet"
                class="shrink-0 text-[10px] tracking-[0.16em]"
                icon="i-lucide-folder-open"
                @click="pickFolder"
            >
              Обзор
            </AppButton>
            <AppButton
                tone="quiet"
                class="shrink-0 text-[10px] tracking-[0.16em]"
                icon="i-lucide-refresh-cw"
                :loading="scanning"
                :disabled="!folder.trim()"
                @click="scan"
            >
              Искать
            </AppButton>
          </div>
        </div>

        <div class="flex items-center gap-4">
          <span
              class="font-mono text-[10px] uppercase tracking-[0.2em]"
              :class="allFound ? 'text-acid' : 'text-fg-faint'"
          >
            Найдено {{ found.length }} из {{ files.length }}
          </span>
          <span class="h-px flex-1 bg-line"/>
          <AppButton
              v-if="missing.length"
              tone="quiet"
              class="shrink-0 text-[10px] tracking-[0.16em]"
              icon="i-lucide-external-link"
              @click="openMissing"
          >
            Открыть все недостающие
          </AppButton>
        </div>

        <ul class="max-h-72 divide-y divide-line overflow-y-auto border border-line">
          <li v-for="file in files" :key="file.targetPath" class="flex items-center gap-3 px-4 py-3">
            <UIcon
                :name="file.localPath ? 'i-lucide-check' : 'i-lucide-x'"
                class="size-3.5 shrink-0"
                :class="file.localPath ? 'text-acid' : 'text-red-400'"
            />

            <div class="min-w-0 flex-1">
              <p class="truncate text-[12px] text-fg" :title="file.fileName">{{ file.fileName }}</p>
              <p
                  class="mt-1 truncate font-mono text-[10px] text-fg-faint"
                  :title="file.localPath ?? file.targetPath"
              >
                {{ file.localPath ?? file.targetPath }}
              </p>
            </div>

            <AppButton
                v-if="file.websiteUrl && !file.localPath"
                tone="quiet"
                class="shrink-0 text-[10px] tracking-[0.16em]"
                icon="i-lucide-download"
                @click="openFile(file)"
            >
              Скачать
            </AppButton>
          </li>
        </ul>

        <p v-if="missing.length" class="text-[12px] leading-relaxed text-fg-muted">
          Недостающие файлы отметятся сами, как только докачаются. Если браузер складывает их не в эту
          папку - укажите нужную. Можно продолжить и без них: тогда пак встанет неполным, а список
          останется во вкладке «Модпак».
        </p>

        <p v-else-if="allFound" class="text-[12px] leading-relaxed text-fg-muted">
          Всё на месте. Дальше лаунчер докачает остальные файлы пака сам - больше он не остановится.
        </p>

        <div class="flex items-center justify-between gap-4 border-t border-line pt-5">
          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.16em] text-fg-faint hover:text-red-400"
              icon="i-lucide-circle-stop"
              @click="cancel"
          >
            Прервать установку
          </AppButton>

          <AppButton
              class="h-10 px-6 tracking-[0.18em]"
              :loading="finishing"
              @click="finish"
          >
            {{ allFound ? 'Продолжить' : 'Продолжить без них' }}
          </AppButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
