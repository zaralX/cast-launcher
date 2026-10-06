<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_DIR_KEYS, INSTANCE_DIRS, INSTANCE_TYPE_LABELS } from '~/types/instance'

const props = defineProps<{
  instance: Instance | null
}>()

const emit = defineEmits<{
  remove: [id: string]
}>()

const actions = useInstanceActions()

const exportOpen = ref(false)

// The panel stays mounted while the selection moves: a dialog of the previous instance closes.
watch(() => props.instance?.id, () => {
  exportOpen.value = false
})

const state = computed(() => props.instance ? actions.stateOf(props.instance) : null)
const install = computed(() => props.instance ? actions.installOf(props.instance.id) : undefined)

const { total } = usePlaytime(() => props.instance)

const { t } = useI18n()

const playtime = computed(() => formatPlaytime(total.value) || t('instance.playtime.never'))
const lastPlayed = computed(() => formatLastPlayed(props.instance?.playtime?.lastPlayedAt ?? 0))

const DIR_ICONS: Record<typeof INSTANCE_DIRS[number], string> = {
  root: 'i-lucide-folder-open',
  minecraft: 'i-lucide-box',
  logs: 'i-lucide-scroll-text',
}
</script>

<template>
  <aside class="flex flex-col border-l border-line bg-ink-800">
    <div
      v-if="instance"
      :key="instance.id"
      class="flex min-h-0 flex-1 flex-col"
    >
      <div class="min-h-0 flex-1 overflow-y-auto">
        <header class="flex flex-col items-center gap-3 border-b border-line px-5 py-6 text-center">
          <InstanceIcon
            :icon="instance.icon"
            :type="instance.type"
            size="lg"
            class="text-fg-faint"
          />

          <div class="w-full min-w-0">
            <h3
              class="font-unbounded text-lead font-semibold leading-tight break-words tracking-heading text-fg"
              :title="instance.name"
            >
              {{ instance.name }}
            </h3>
            <p class="mt-2 font-mono text-micro uppercase tracking-caps text-fg-faint">
              {{ INSTANCE_TYPE_LABELS[instance.type] ?? instance.type }} · {{ instance.minecraftVersion }}
            </p>
          </div>
        </header>

        <div class="space-y-3 px-5 py-5">
          <InstanceInstallProgress
            v-if="state === 'installing'"
            :phase="install?.phase"
            :progress="install?.progress"
            class="h-9"
          />

          <Button
            v-else-if="state !== 'running'"
            size="sm"
            class="group/act w-full"
            @click="actions.primary(instance)"
          >
            <Icon
              :name="state === 'ready' ? 'i-lucide-play' : 'i-lucide-arrow-down-to-line'"
              class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-0.5"
            />
            {{ state === 'ready' ? $t('instance.action.play') : $t('instance.action.install') }}
          </Button>

          <Button
            v-if="state === 'running'"
            variant="fill-accent"
            size="sm"
            icon="i-lucide-square"
            class="w-full"
            @click="actions.stop(instance.id)"
          >
            {{ $t('instance.panel.stop') }}
          </Button>

          <Button
            v-if="state === 'installing'"
            size="sm"
            icon="i-lucide-x"
            class="w-full"
            :disabled="install?.aborting"
            @click="actions.cancelInstall(instance.id)"
          >
            {{ install?.aborting ? $t('instance.panel.aborting') : $t('instance.panel.cancel') }}
          </Button>

          <Button
            :to="`/instance/${instance.id}`"
            size="sm"
            icon="i-lucide-settings"
            class="w-full"
          >
            {{ $t('instance.action.settings') }}
          </Button>

          <Button
            size="sm"
            icon="i-lucide-file-down"
            class="w-full"
            :disabled="state === 'installing'"
            @click="exportOpen = true"
          >
            {{ $t('instance.export_cast') }}
          </Button>

          <div class="grid grid-cols-3 gap-2">
            <Button
              v-for="target in INSTANCE_DIRS"
              :key="target"
              variant="outline-accent"
              size="sm"
              :icon="DIR_ICONS[target]"
              :title="$t(INSTANCE_DIR_KEYS[target])"
              :aria-label="$t(INSTANCE_DIR_KEYS[target])"
              @click="actions.openDir(instance.id, target)"
            />
          </div>
        </div>

        <dl class="space-y-2 border-t border-line px-5 py-4 font-mono text-label uppercase tracking-caps">
          <div>
            <dt class="text-fg-faint">
              {{ $t('instance.panel.playtime') }}
            </dt>
            <dd class="min-w-0 truncate text-fg-muted">
              {{ playtime }}
            </dd>
          </div>
          <div v-if="lastPlayed">
            <dt class="text-fg-faint">
              {{ $t('instance.panel.last_played') }}
            </dt>
            <dd class="min-w-0 truncate text-fg-muted">
              {{ lastPlayed }}
            </dd>
          </div>
          <div
            v-if="instance.castpack"
            class="flex items-baseline justify-between gap-3"
          >
            <dt class="text-fg-faint">
              {{ instance.castpack.origin === 'file' ? '.cast' : 'CastPack' }}
            </dt>
            <dd class="min-w-0 truncate text-fg-muted">
              {{ instance.castpack.version || '—' }}
            </dd>
          </div>
        </dl>
      </div>

      <div class="border-t border-line px-5 py-4">
        <Button
          variant="quiet-danger"
          icon="i-lucide-trash-2"
          @click="emit('remove', instance.id)"
        >
          {{ $t('instance.panel.remove') }}
        </Button>
      </div>

      <CastExportModal
        v-model:open="exportOpen"
        :instance="instance"
      />
    </div>

    <div
      v-else
      class="flex flex-1 flex-col items-center justify-center gap-3 px-5 py-14 text-center"
    >
      <Icon
        name="i-lucide-mouse-pointer-click"
        class="size-5 text-fg-faint"
      />
      <i18n-t
        keypath="instance.panel.empty"
        tag="p"
        class="font-mono text-label uppercase leading-relaxed tracking-caps text-fg-faint"
      >
        <template #br>
          <br>
        </template>
      </i18n-t>
    </div>
  </aside>
</template>
