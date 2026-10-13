<script setup lang="ts">
import type { Instance } from '~/types/instance'

const props = defineProps<{
  instance: Instance
  selected?: boolean
}>()

const emit = defineEmits<{
  select: [id: string]
  export: [id: string]
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
  <InstanceActionsMenu
    :instance="instance"
    @export="emit('export', $event)"
    @remove="emit('remove', $event)"
  >
    <KitTile
      variant="ghost"
      :selected="selected"
      :title="instance.name"
      class="flex w-full flex-col items-center gap-1.75 px-1 pt-2.5 pb-2.25 text-center"
      @click="select"
      @dblclick="actions.primary(instance)"
      @contextmenu="select"
    >
      <span class="relative">
        <InstanceIcon
          :icon="instance.icon"
          :type="instance.type"
          :version="instance.minecraftVersion"
          size="md"
        />
        <span
          v-if="state === 'running'"
          class="absolute -top-1 -right-1 size-3 rounded-pill bg-success ring-3 ring-sheet"
          :title="$t('instance.state.running')"
        />
      </span>

      <Progress
        v-if="state === 'installing'"
        :model-value="install?.progress == null ? null : install.progress * 100"
        class="-mt-0.75 h-0.75 w-12"
      />

      <span
        class="line-clamp-2 max-w-full text-caption leading-tight break-words"
        :class="state === 'absent' ? 'text-fg-faint' : 'text-fg'"
      >
        {{ instance.name }}
      </span>
    </KitTile>
  </InstanceActionsMenu>
</template>
