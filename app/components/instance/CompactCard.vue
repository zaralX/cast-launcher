<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_DIR_KEYS, INSTANCE_DIRS } from '~/types/instance'

const props = defineProps<{
  instance: Instance
  selected?: boolean
}>()

const emit = defineEmits<{
  select: [id: string]
  remove: [id: string]
}>()

const actions = useInstanceActions()

const state = computed(() => actions.stateOf(props.instance))
const install = computed(() => actions.installOf(props.instance.id))

function select() {
  emit('select', props.instance.id)
}
</script>

<template>
  <ContextMenu>
    <ContextMenuTrigger as-child>
      <button
        type="button"
        :aria-pressed="!!selected"
        :title="instance.name"
        class="group relative flex w-full cursor-pointer flex-col items-center gap-2 border p-2.5 text-center outline-none transition-colors duration-300 focus-visible:border-line-strong"
        :class="selected
          ? 'border-acid bg-ink-700'
          : 'border-transparent hover:border-line-strong hover:bg-ink-800'"
        @click="select"
        @dblclick="actions.primary(instance)"
        @contextmenu="select"
      >
        <span class="relative">
          <InstanceIcon
            :icon="instance.icon"
            :type="instance.type"
            size="md"
            :bordered="false"
            class="text-fg-faint transition-colors duration-300"
            :class="selected ? 'text-acid' : 'group-hover:text-fg-muted'"
          />

          <KitLiveDot
            v-if="state === 'running'"
            class="absolute -top-1 -right-1 text-acid"
            :title="$t('instance.state.running')"
          />

          <span
            v-else-if="state === 'absent'"
            class="absolute -top-1 -right-1 size-2 bg-fg-faint"
            :title="$t('instance.state.absent')"
          />
        </span>

        <span
          class="line-clamp-2 w-full break-words text-caption leading-tight transition-colors duration-300"
          :class="selected ? 'text-fg' : 'text-fg-muted group-hover:text-fg'"
        >
          {{ instance.name }}
        </span>

        <Progress
          v-if="state === 'installing'"
          :model-value="install?.progress == null ? null : install.progress * 100"
        />
      </button>
    </ContextMenuTrigger>

    <ContextMenuContent>
      <ContextMenuItem
        v-if="state === 'running'"
        icon="i-lucide-square"
        @select="actions.stop(instance.id)"
      >
        {{ $t('instance.action.stop') }}
      </ContextMenuItem>
      <ContextMenuItem
        v-else-if="state === 'installing'"
        icon="i-lucide-x"
        @select="actions.cancelInstall(instance.id)"
      >
        {{ $t('instance.action.cancel_install') }}
      </ContextMenuItem>
      <ContextMenuItem
        v-else-if="state === 'ready'"
        icon="i-lucide-play"
        @select="actions.play(instance.id)"
      >
        {{ $t('instance.action.play') }}
      </ContextMenuItem>
      <ContextMenuItem
        v-else
        icon="i-lucide-arrow-down-to-line"
        @select="actions.install(instance.id)"
      >
        {{ $t('instance.action.install') }}
      </ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem
        icon="i-lucide-settings"
        @select="navigateTo(`/instance/${instance.id}`)"
      >
        {{ $t('instance.action.settings') }}
      </ContextMenuItem>
      <ContextMenuItem
        v-for="target in INSTANCE_DIRS"
        :key="target"
        icon="i-lucide-folder-open"
        @select="actions.openDir(instance.id, target)"
      >
        {{ $t(INSTANCE_DIR_KEYS[target]) }}
      </ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem
        icon="i-lucide-trash-2"
        variant="danger"
        @select="emit('remove', instance.id)"
      >
        {{ $t('instance.action.remove') }}
      </ContextMenuItem>
    </ContextMenuContent>
  </ContextMenu>
</template>
