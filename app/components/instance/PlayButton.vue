<script setup lang="ts">
import type { Instance } from '~/types/instance'

const props = withDefaults(defineProps<{
  instance: Instance
  size?: 'md' | 'lg'
}>(), {
  size: 'md',
})

const actions = useInstanceActions()

const state = computed(() => actions.stateOf(props.instance))
const install = computed(() => actions.installOf(props.instance.id))
const percent = computed(() => Math.round((install.value?.progress ?? 0) * 100))

const width = computed(() => (props.size === 'lg' ? 'min-w-37' : 'min-w-28'))
</script>

<template>
  <Button
    v-if="state === 'running'"
    :size="size"
    icon="i-lucide-square"
    :class="width"
    @click="actions.stop(instance.id)"
  >
    {{ $t('instance.action.stop') }}
  </Button>

  <Button
    v-else-if="state === 'installing'"
    :size="size"
    icon="i-lucide-x"
    :disabled="install?.aborting"
    :title="install?.phase ? uiText(install.phase) : undefined"
    :class="width"
    @click="actions.cancelInstall(instance.id)"
  >
    <span class="tabular-nums">{{ percent }}%</span>
    · {{ $t('instance.panel.cancel') }}
  </Button>

  <Button
    v-else
    variant="primary"
    :size="size"
    :icon="state === 'ready' ? 'i-lucide-play' : 'i-lucide-arrow-down-to-line'"
    :class="width"
    @click="actions.primary(instance)"
  >
    {{ state === 'ready' ? $t('instance.action.play') : $t('instance.action.install') }}
  </Button>
</template>
