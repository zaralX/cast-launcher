<script setup lang="ts">
import type { Instance } from '~/types/instance'
import type { CastPackUpdate } from '~/types/castpack'

const props = defineProps<{ instance: Instance }>()

const instanceStore = useInstanceStore()
const castpackStore = useCastPackStore()
const toast = useToast()

const checking = ref(false)
const update = ref<CastPackUpdate | null>(null)
const updateFromFile = ref(false)

const source = computed(() => props.instance.castpack)
const fromFile = computed(() => source.value?.origin === 'file')
const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const autoupdate = computed({
  get: () => source.value?.autoupdate ?? false,
  set: (enabled: boolean) => toggle(enabled),
})

const { t } = useI18n()

const blocked = computed(() => {
  if (running.value) return t('instance.pack.blocked_running')
  if (installing.value) return t('instance.pack.blocked_installing')
  return null
})

const facts = computed(() => {
  const version = {
    label: t('instance.castpack.facts.version'),
    value: source.value?.version || t('instance.castpack.facts.version_absent'),
  }

  if (fromFile.value) {
    return [
      { label: t('instance.castpack.facts.pack_id'), value: source.value?.catalogId ?? '-' },
      version,
      { label: t('instance.castpack.facts.source'), value: t('instance.castpack.facts.source_file') },
    ]
  }

  return [
    { label: t('instance.castpack.facts.pack'), value: source.value?.catalogId ?? '-' },
    version,
    {
      label: t('instance.castpack.facts.autoupdate'),
      value: source.value?.autoupdate
        ? t('instance.castpack.facts.autoupdate_on')
        : t('instance.castpack.facts.autoupdate_off'),
    },
    { label: t('instance.castpack.facts.manifest'), value: source.value?.manifestUrl ?? '-' },
  ]
})

const announceImport = useImportToast()

function onUpdated(instanceId: string, updated: boolean) {
  updateFromFile.value = false
  announceImport(instanceId, updated)
}

async function check() {
  if (checking.value || fromFile.value) return

  checking.value = true

  const result = await attempt(() => castpackStore.checkUpdate(props.instance.id), {
    code: 'NETWORK',
    context: { instanceId: props.instance.id, action: t('instance.castpack.check_action') },
  })

  checking.value = false

  if (result.ok) update.value = result.value
}

onMounted(check)

async function toggle(enabled: boolean) {
  const result = await attempt(() => castpackStore.setAutoupdate(props.instance.id, enabled), {
    context: { instanceId: props.instance.id, action: t('instance.castpack.toggle_action') },
  })

  if (!result.ok) return

  toast.add({
    title: enabled ? t('instance.castpack.autoupdate_on_title') : t('instance.castpack.autoupdate_off_title'),
    description: enabled
      ? t('instance.castpack.autoupdate_on_hint')
      : t('instance.castpack.autoupdate_off_hint'),
    color: 'success',
    icon: 'i-lucide-refresh-cw',
  })
}

async function reinstall() {
  if (blocked.value) return

  const started = await attempt(() => instanceStore.installInstance(props.instance.id), {
    code: 'NETWORK',
    context: { instanceId: props.instance.id, action: t('instance.castpack.reinstall_action') },
  })

  if (!started.ok) return

  toast.add({
    title: t('instance.castpack.repair_title'),
    description: t('instance.castpack.repair_hint'),
    color: 'success',
    icon: 'i-lucide-refresh-cw',
  })
}

const openSite = (url: string) => safeRun(() => call('open_url', { url }))
</script>

<template>
  <div class="space-y-6">
    <SettingsPanel
      index="01"
      :title="$t('instance.castpack.title')"
      icon="i-lucide-layers"
    >
      <div
        v-if="!source"
        class="text-[12px] leading-relaxed text-fg-muted"
      >
        {{ $t('instance.castpack.not_castpack') }}
      </div>

      <div
        v-else-if="fromFile"
        class="space-y-7"
      >
        <p class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted">
          <UIcon
            name="i-lucide-file-archive"
            class="mt-0.5 size-3.5 shrink-0 text-acid"
          />
          {{ $t('instance.castpack.file_hint') }}
        </p>

        <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
          <p class="min-w-0 text-[12px] leading-relaxed text-fg-muted">
            {{ $t('instance.castpack.files_hint') }}
          </p>

          <div class="flex shrink-0 gap-2">
            <AppButton
              tone="quiet"
              class="h-9 text-[10px] tracking-[0.18em]"
              icon="i-lucide-wrench"
              :disabled="!!blocked"
              @click="reinstall"
            >
              {{ $t('instance.castpack.repair') }}
            </AppButton>

            <AppButton
              class="h-9 text-[10px] tracking-[0.18em]"
              icon="i-lucide-file-up"
              :disabled="!!blocked"
              @click="updateFromFile = true"
            >
              {{ $t('instance.castpack.update_from_file') }}
            </AppButton>
          </div>
        </div>

        <p
          v-if="blocked"
          class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint"
        >
          {{ blocked }}
        </p>
      </div>

      <div
        v-else
        class="space-y-7"
      >
        <div
          v-if="update?.available"
          class="flex items-start justify-between gap-4 border border-acid/30 bg-acid/[0.04] px-4 py-3"
        >
          <div class="min-w-0">
            <p class="text-[12px] leading-relaxed text-fg-muted">
              <i18n-t
                keypath="instance.castpack.update_available"
                tag="span"
              >
                <template #version>
                  <span class="text-fg">{{ update.version }}</span>
                </template>
              </i18n-t>
              {{ $t('instance.castpack.update_auto') }}
            </p>
            <p
              v-if="update.changelog"
              class="mt-2 whitespace-pre-line text-[12px] leading-relaxed text-fg-muted"
            >
              {{ update.changelog }}
            </p>
          </div>

          <AppButton
            tone="quiet"
            class="shrink-0 text-[10px] tracking-[0.18em]"
            icon="i-lucide-arrow-down-to-line"
            :disabled="!!blocked"
            @click="reinstall"
          >
            {{ $t('instance.castpack.update') }}
          </AppButton>
        </div>

        <p
          v-else-if="update?.error"
          class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
        >
          <UIcon
            name="i-lucide-wifi-off"
            class="mt-0.5 size-3.5 shrink-0 text-amber-400"
          />
          {{ $t('instance.castpack.check_failed', { error: uiText(update.error.text) }) }}
        </p>

        <p
          v-else-if="update"
          class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
        >
          <UIcon
            name="i-lucide-check"
            class="mt-0.5 size-3.5 shrink-0 text-acid"
          />
          {{ $t('instance.castpack.up_to_date') }}
        </p>

        <SettingsField
          :label="$t('instance.castpack.autoupdate')"
          :hint="$t('instance.castpack.autoupdate_hint')"
        >
          <USwitch v-model="autoupdate" />
        </SettingsField>

        <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
          <p class="min-w-0 text-[12px] leading-relaxed text-fg-muted">
            {{ $t('instance.castpack.files_hint') }}
          </p>

          <div class="flex shrink-0 gap-2">
            <AppButton
              tone="quiet"
              class="h-9 text-[10px] tracking-[0.18em]"
              icon="i-lucide-rotate-cw"
              :loading="checking"
              @click="check"
            >
              {{ $t('instance.castpack.check') }}
            </AppButton>

            <AppButton
              class="h-9 text-[10px] tracking-[0.18em]"
              icon="i-lucide-wrench"
              :disabled="!!blocked"
              @click="reinstall"
            >
              {{ $t('instance.castpack.repair') }}
            </AppButton>
          </div>
        </div>

        <p
          v-if="blocked"
          class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint"
        >
          {{ blocked }}
        </p>
      </div>
    </SettingsPanel>

    <SettingsPanel
      v-if="source?.changelog"
      index="02"
      :title="$t('instance.castpack.changelog_title')"
      icon="i-lucide-scroll-text"
    >
      <p class="whitespace-pre-line text-[12px] leading-relaxed text-fg-muted">
        {{ source.changelog }}
      </p>
    </SettingsPanel>

    <SettingsPanel
      :index="source?.changelog ? '03' : '02'"
      :title="$t('instance.castpack.source_title')"
      icon="i-lucide-link"
    >
      <dl class="grid gap-x-6 gap-y-4 sm:grid-cols-2">
        <div
          v-for="fact in facts"
          :key="fact.label"
          class="min-w-0"
        >
          <dt class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            {{ fact.label }}
          </dt>
          <dd
            class="mt-1.5 truncate font-mono text-[12px] text-fg-muted"
            :title="fact.value"
          >
            {{ fact.value }}
          </dd>
        </div>
      </dl>

      <AppButton
        v-if="source?.manifestUrl && !fromFile"
        tone="quiet"
        class="mt-5 text-[10px] tracking-[0.16em]"
        icon="i-lucide-external-link"
        @click="openSite(source.manifestUrl)"
      >
        {{ $t('instance.castpack.open_manifest') }}
      </AppButton>
    </SettingsPanel>

    <UModal
      v-model:open="updateFromFile"
      :title="$t('instance.castpack.update_from_file_title', { name: instance.name })"
    >
      <template #body>
        <ImportPackModalBody
          :update-target="instance.id"
          @imported="onUpdated"
        />
      </template>
    </UModal>
  </div>
</template>
