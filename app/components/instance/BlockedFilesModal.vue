<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { BlockedFile } from '~/types/catalog'
import type { InstallSnapshot } from '~/types/instance'

const FOCUS_COOLDOWN = 2000

const { t } = useI18n()

const props = defineProps<{ install: InstallSnapshot }>()

const files = ref<BlockedFile[]>(props.install.blocked ?? [])
const folder = ref('')
const scanning = ref(false)
const finishing = ref(false)

const missing = computed(() => files.value.filter(file => !file.localPath))
const found = computed(() => files.value.filter(file => file.localPath))
const allFound = computed(() => files.value.length > 0 && missing.value.length === 0)

let lastRescan = 0

watch(() => props.install.blocked, (value) => {
  if (value?.length) files.value = value
})

onMounted(async () => {
  folder.value = await safeRun(() => call('downloads_dir')) ?? ''

  const current = await safeRun(() => call('awaited_files', { instanceId: props.install.instanceId }))
  if (current?.length) files.value = current

  const unlisten = await safeRun(() => getCurrentWindow().onFocusChanged(({ payload }) => {
    if (payload) rescan()
  }))

  if (unlisten) onScopeDispose(unlisten)
})

async function rescan() {
  if (scanning.value || allFound.value) return
  if (Date.now() - lastRescan < FOCUS_COOLDOWN) return

  lastRescan = Date.now()
  scanning.value = true

  const result = await call('rescan_files', { instanceId: props.install.instanceId }).catch(() => null)
  if (result?.length) files.value = result
  scanning.value = false
}

async function scan() {
  if (!folder.value.trim() || scanning.value) return

  lastRescan = Date.now()
  scanning.value = true

  const result = await safeRun(
    () => call('scan_for_files', { instanceId: props.install.instanceId, folder: folder.value }),
    { context: { action: t('blocked.scan_action'), instanceId: props.install.instanceId } },
  )

  if (result?.length) files.value = result

  scanning.value = false
}

async function pickFolder() {
  const picked = await safeRun(() => call('pick_folder', { title: t('blocked.picker_title') }))

  if (!picked) return

  folder.value = picked
  await scan()
}

const openFile = (file: BlockedFile) => safeRun(() => call('open_url', { url: file.websiteUrl }))

async function openMissing() {
  for (const file of missing.value) {
    if (file.websiteUrl) await openFile(file)
  }
}

async function finish() {
  finishing.value = true
  await safeRun(() => call('resume_install', { instanceId: props.install.instanceId }))
  finishing.value = false
}

const cancel = () => safeRun(() => call('cancel_install', { instanceId: props.install.instanceId }))
</script>

<template>
  <Dialog :open="true">
    <DialogContent
      :dismissible="false"
      :show-close-button="false"
      class="max-w-170"
    >
      <DialogHeader>
        <DialogTitle>{{ $t('blocked.title') }}</DialogTitle>
        <DialogDescription class="truncate text-caption">
          {{ install.instanceName }}
        </DialogDescription>
      </DialogHeader>

      <DialogBody class="space-y-5">
        <p class="text-body text-fg-muted">
          {{ $t('blocked.description') }}
        </p>

        <div>
          <div class="mb-1.5 flex items-center gap-2">
            <Label for="blocked-folder">
              {{ $t('blocked.where') }}
            </Label>
            <span
              v-if="!allFound"
              class="flex items-center gap-1.5 text-caption text-accent"
            >
              <KitLiveDot size="sm" />
              {{ $t('blocked.auto') }}
            </span>
          </div>
          <div class="flex gap-2">
            <Input
              id="blocked-folder"
              v-model="folder"
              font="mono"
              :placeholder="$t('blocked.folder_placeholder')"
              class="flex-1"
              @keydown.enter.prevent="scan"
            />
            <Button
              icon="i-lucide-folder-open"
              @click="pickFolder"
            >
              {{ $t('blocked.browse') }}
            </Button>
            <Button
              variant="ghost"
              icon="i-lucide-refresh-cw"
              :loading="scanning"
              :disabled="!folder.trim()"
              @click="scan"
            >
              {{ $t('blocked.scan') }}
            </Button>
          </div>
        </div>

        <div>
          <div class="mb-1.5 flex items-center justify-between gap-4">
            <span
              class="text-caption font-medium"
              :class="allFound ? 'text-success' : 'text-fg-muted'"
            >
              {{ $t('blocked.found', { found: found.length, total: files.length }) }}
            </span>
            <Button
              v-if="missing.length"
              variant="ghost"
              size="sm"
              icon="i-lucide-external-link"
              class="-mr-2.5"
              @click="openMissing"
            >
              {{ $t('blocked.open_missing') }}
            </Button>
          </div>

          <ul class="max-h-72 divide-y divide-line overflow-y-auto rounded-md bg-panel ring-1 ring-card-line ring-inset">
            <li
              v-for="file in files"
              :key="file.targetPath"
              class="flex items-center gap-3 px-3.5 py-2.5"
            >
              <Icon
                :name="file.localPath ? 'i-lucide-check' : 'i-lucide-minus'"
                class="size-4 shrink-0"
                :class="file.localPath ? 'text-success' : 'text-fg-faint'"
              />
              <div class="min-w-0 flex-1">
                <p
                  class="truncate text-fg"
                  :title="file.fileName"
                >
                  {{ file.fileName }}
                </p>
                <p
                  class="truncate font-mono text-caption text-fg-faint"
                  :title="file.localPath ?? file.targetPath"
                >
                  {{ file.localPath ?? file.targetPath }}
                </p>
              </div>
              <Button
                v-if="file.websiteUrl && !file.localPath"
                variant="ghost"
                size="sm"
                icon="i-lucide-external-link"
                @click="openFile(file)"
              >
                {{ $t('blocked.download') }}
              </Button>
            </li>
          </ul>
        </div>
      </DialogBody>

      <DialogFooter class="border-t border-line pt-3.5 pb-3.5">
        <Button
          variant="danger-ghost"
          icon="i-lucide-circle-stop"
          class="mr-auto"
          @click="cancel"
        >
          {{ $t('blocked.cancel') }}
        </Button>

        <Button
          variant="primary"
          :loading="finishing"
          @click="finish"
        >
          {{ allFound ? $t('blocked.continue') : $t('blocked.continue_without') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
