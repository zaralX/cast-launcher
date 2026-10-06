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
      class="max-w-2xl"
    >
      <DialogHeader>
        <DialogTitle>{{ $t('blocked.title') }}</DialogTitle>
      </DialogHeader>

      <DialogBody class="space-y-6">
        <DialogDescription>
          {{ $t('blocked.description') }}
        </DialogDescription>

        <div>
          <Label
            for="blocked-folder"
            class="mb-2 flex items-center gap-2"
          >
            <span>{{ $t('blocked.where') }}</span>
            <span
              v-if="!allFound"
              class="flex items-center gap-1.5 text-acid/70"
            >
              <span class="size-1 animate-pulse rounded-full bg-acid" />
              {{ $t('blocked.auto') }}
            </span>
          </Label>

          <div class="flex gap-4">
            <Input
              id="blocked-folder"
              v-model="folder"
              :placeholder="$t('blocked.folder_placeholder')"
              class="flex-1"
              @keydown.enter.prevent="scan"
            />
            <Button
              variant="quiet"
              icon="i-lucide-folder-open"
              @click="pickFolder"
            >
              {{ $t('blocked.browse') }}
            </Button>
            <Button
              variant="quiet"
              icon="i-lucide-refresh-cw"
              :loading="scanning"
              :disabled="!folder.trim()"
              @click="scan"
            >
              {{ $t('blocked.scan') }}
            </Button>
          </div>
        </div>

        <div class="flex items-center gap-4">
          <span
            class="font-mono text-label uppercase tracking-caps"
            :class="allFound ? 'text-acid' : 'text-fg-faint'"
          >
            {{ $t('blocked.found', { found: found.length, total: files.length }) }}
          </span>
          <Separator class="flex-1" />
          <Button
            v-if="missing.length"
            variant="quiet"
            icon="i-lucide-external-link"
            @click="openMissing"
          >
            {{ $t('blocked.open_missing') }}
          </Button>
        </div>

        <ul class="max-h-72 divide-y divide-line overflow-y-auto border border-line">
          <li
            v-for="file in files"
            :key="file.targetPath"
            class="flex items-center gap-3 px-4 py-3"
          >
            <Icon
              :name="file.localPath ? 'i-lucide-check' : 'i-lucide-x'"
              class="size-3.5 shrink-0"
              :class="file.localPath ? 'text-acid' : 'text-danger'"
            />

            <div class="min-w-0 flex-1">
              <p
                class="truncate text-body text-fg"
                :title="file.fileName"
              >
                {{ file.fileName }}
              </p>
              <p
                class="mt-1 truncate font-mono text-label text-fg-faint"
                :title="file.localPath ?? file.targetPath"
              >
                {{ file.localPath ?? file.targetPath }}
              </p>
            </div>

            <Button
              v-if="file.websiteUrl && !file.localPath"
              variant="quiet"
              icon="i-lucide-download"
              @click="openFile(file)"
            >
              {{ $t('blocked.download') }}
            </Button>
          </li>
        </ul>

        <p
          v-if="missing.length"
          class="text-body leading-relaxed text-fg-muted"
        >
          {{ $t('blocked.missing_hint') }}
        </p>

        <p
          v-else-if="allFound"
          class="text-body leading-relaxed text-fg-muted"
        >
          {{ $t('blocked.all_found_hint') }}
        </p>
      </DialogBody>

      <DialogFooter class="justify-between">
        <Button
          variant="quiet-danger"
          icon="i-lucide-circle-stop"
          @click="cancel"
        >
          {{ $t('blocked.cancel') }}
        </Button>

        <Button
          size="lg"
          :loading="finishing"
          @click="finish"
        >
          {{ allFound ? $t('blocked.continue') : $t('blocked.continue_without') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
