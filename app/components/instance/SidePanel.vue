<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_DIR_KEYS, INSTANCE_DIRS, INSTANCE_TYPE_LABELS } from '~/types/instance'

const props = defineProps<{
  instance: Instance | null
}>()

const emit = defineEmits<{
  export: [id: string]
  remove: [id: string]
}>()

const actions = useInstanceActions()
const appStore = useAppStore()

const state = computed(() => props.instance ? actions.stateOf(props.instance) : null)
const install = computed(() => props.instance ? actions.installOf(props.instance.id) : undefined)

const { total } = usePlaytime(() => props.instance)

const { t } = useI18n()

const playtime = computed(() => formatPlaytime(total.value) || t('instance.playtime.never'))
const lastPlayed = computed(() => formatLastPlayed(props.instance?.playtime?.lastPlayedAt ?? 0))

const memory = computed(() => {
  const settings = props.instance?.settings
  const megabytes = settings?.overrideMemory ? settings.maxRam : appStore.config?.java.max_ram
  return megabytes ? formatBytes(megabytes * 1024 * 1024) : ''
})
</script>

<template>
  <aside class="flex flex-col border-l border-line bg-panel">
    <div
      v-if="instance"
      :key="instance.id"
      class="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 pt-7 pb-5.5"
    >
      <header class="flex items-center gap-3.5">
        <InstanceIcon
          :icon="instance.icon"
          :type="instance.type"
          :version="instance.minecraftVersion"
          size="xl"
        />

        <div class="min-w-0">
          <NuxtLink
            :to="`/instance/${instance.id}`"
            class="line-clamp-2 font-unbounded text-heading font-semibold tracking-heading break-words text-fg hover:underline classic:uppercase"
            :title="instance.name"
          >
            {{ instance.name }}
          </NuxtLink>
          <p
            v-if="instance.castpack"
            class="mt-1.5 truncate text-caption text-fg-muted"
          >
            {{ instance.castpack.origin === 'file' ? '.cast' : 'CastPack' }} {{ instance.castpack.version }}
          </p>
          <p class="truncate text-caption text-fg-faint">
            {{ INSTANCE_TYPE_LABELS[instance.type] ?? instance.type }}
            <template v-if="instance.loaderVersion">
              {{ instance.loaderVersion }}
            </template>
            · {{ instance.minecraftVersion }}
          </p>
        </div>
      </header>

      <div class="mt-5">
        <InstanceInstallProgress
          v-if="state === 'installing'"
          :phase="install?.phase"
          :progress="install?.progress"
          class="h-10"
        />
        <Button
          v-else-if="state === 'running'"
          size="lg"
          icon="i-lucide-square"
          class="w-full"
          @click="actions.stop(instance.id)"
        >
          {{ $t('instance.panel.stop') }}
        </Button>
        <Button
          v-else
          variant="primary"
          size="lg"
          :icon="state === 'ready' ? 'i-lucide-play' : 'i-lucide-arrow-down-to-line'"
          class="w-full"
          @click="actions.primary(instance)"
        >
          {{ state === 'ready' ? $t('instance.action.play') : $t('instance.action.install') }}
        </Button>
      </div>

      <div class="mt-2 flex gap-1.5">
        <Button
          v-if="state === 'installing'"
          icon="i-lucide-x"
          class="flex-1"
          :disabled="install?.aborting"
          @click="actions.cancelInstall(instance.id)"
        >
          {{ install?.aborting ? $t('instance.panel.aborting') : $t('instance.panel.cancel') }}
        </Button>
        <Button
          v-else
          :to="`/instance/${instance.id}?tab=settings`"
          icon="i-lucide-settings"
          class="flex-1"
        >
          {{ $t('instance.action.settings') }}
        </Button>

        <Button
          size="icon"
          icon="i-lucide-folder-open"
          :aria-label="$t(INSTANCE_DIR_KEYS.root)"
          :title="$t(INSTANCE_DIR_KEYS.root)"
          @click="actions.openDir(instance.id, 'root')"
        />

        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              size="icon"
              icon="i-lucide-ellipsis"
              :aria-label="$t('instance.panel.more')"
              :title="$t('instance.panel.more')"
            />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              v-for="target in INSTANCE_DIRS.filter(dir => dir !== 'root')"
              :key="target"
              icon="i-lucide-folder-open"
              @select="actions.openDir(instance.id, target)"
            >
              {{ $t(INSTANCE_DIR_KEYS[target]) }}
            </DropdownMenuItem>
            <DropdownMenuItem
              icon="i-lucide-file-down"
              :disabled="state === 'installing'"
              @select="emit('export', instance.id)"
            >
              {{ $t('instance.export_short') }}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              icon="i-lucide-trash-2"
              variant="danger"
              @select="emit('remove', instance.id)"
            >
              {{ $t('instance.panel.remove') }}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <dl class="mt-5 border-t border-line pt-2">
        <KitFact
          :label="$t('instance.panel.playtime')"
          :value="playtime"
        />
        <KitFact
          v-if="lastPlayed"
          :label="$t('instance.panel.last_played')"
          :value="lastPlayed"
        />
        <KitFact
          v-if="memory"
          :label="$t('instance.panel.memory')"
          :value="memory"
        />
      </dl>
    </div>

    <div
      v-else
      class="flex flex-1 flex-col items-center justify-center gap-3 px-6 py-14 text-center text-body text-fg-faint"
    >
      <Icon
        name="i-lucide-mouse-pointer-click"
        class="size-5"
      />
      <p>{{ $t('instance.panel.empty') }}</p>
    </div>
  </aside>
</template>
