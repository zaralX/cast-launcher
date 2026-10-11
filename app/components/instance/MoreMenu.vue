<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_DIR_KEYS } from '~/types/instance'

const props = withDefaults(defineProps<{
  instance: Instance
  size?: 'md' | 'lg'
}>(), {
  size: 'md',
})

const emit = defineEmits<{ export: [] }>()

const actions = useInstanceActions()

const state = computed(() => actions.stateOf(props.instance))
const buttonSize = computed(() => (props.size === 'lg' ? 'icon-lg' : 'icon'))
</script>

<template>
  <Button
    :size="buttonSize"
    icon="i-lucide-folder-open"
    :aria-label="$t(INSTANCE_DIR_KEYS.root)"
    :title="$t(INSTANCE_DIR_KEYS.root)"
    @click="actions.openDir(instance.id, 'root')"
  />

  <DropdownMenu>
    <DropdownMenuTrigger as-child>
      <Button
        :size="buttonSize"
        icon="i-lucide-ellipsis"
        :aria-label="$t('instance.panel.more')"
        :title="$t('instance.panel.more')"
      />
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end">
      <DropdownMenuItem
        icon="i-lucide-file-down"
        :disabled="state === 'installing'"
        @select="emit('export')"
      >
        {{ $t('instance.export_short') }}
      </DropdownMenuItem>
      <DropdownMenuSeparator />
      <DropdownMenuItem
        icon="i-lucide-folder-open"
        @select="actions.openDir(instance.id, 'minecraft')"
      >
        {{ $t(INSTANCE_DIR_KEYS.minecraft) }}
      </DropdownMenuItem>
      <DropdownMenuItem
        icon="i-lucide-folder-clock"
        @select="actions.openDir(instance.id, 'logs')"
      >
        {{ $t(INSTANCE_DIR_KEYS.logs) }}
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
</template>
