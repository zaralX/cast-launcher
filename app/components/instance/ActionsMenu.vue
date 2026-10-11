<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_DIR_KEYS, INSTANCE_DIRS } from '~/types/instance'

const props = defineProps<{
  instance: Instance
}>()

const emit = defineEmits<{
  export: [id: string]
  remove: [id: string]
}>()

const actions = useInstanceActions()

const state = computed(() => actions.stateOf(props.instance))
</script>

<template>
  <ContextMenu>
    <ContextMenuTrigger as-child>
      <slot />
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
      <ContextMenuItem
        icon="i-lucide-file-down"
        :disabled="state === 'installing'"
        @select="emit('export', instance.id)"
      >
        {{ $t('instance.export_short') }}
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
