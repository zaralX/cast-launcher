<script setup lang="ts">
import type { Instance } from '~/types/instance'
import { INSTANCE_TYPE_LABELS } from '~/types/instance'
import { FOCUS_RING } from '@/lib/styles'

const props = defineProps<{
  instance: Instance
}>()

const emit = defineEmits<{
  export: [id: string]
  remove: [id: string]
}>()

const actions = useInstanceActions()

const state = computed(() => actions.stateOf(props.instance))
const install = computed(() => actions.installOf(props.instance.id))

const { total, session } = usePlaytime(() => props.instance)

const { t } = useI18n()

const loader = computed(() => `${INSTANCE_TYPE_LABELS[props.instance.type] ?? props.instance.type} ${props.instance.minecraftVersion}`)

const playtime = computed(() => {
  if (!props.instance.installed) return t('instance.state.absent').toLowerCase()
  return formatPlaytime(total.value) || t('instance.playtime.never')
})

const clock = computed(() => {
  const minutes = Math.floor(session.value / 60)
  return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, '0')}`
})

const percent = computed(() => install.value?.progress == null ? null : Math.round(install.value.progress * 100))
</script>

<template>
  <InstanceActionsMenu
    :instance="instance"
    @export="emit('export', $event)"
    @remove="emit('remove', $event)"
  >
    <article
      class="group/card relative flex h-19 items-center gap-3.5 overflow-hidden rounded-md bg-panel py-3 pr-3.5 pl-3 ring-1 ring-inset transition-shadow duration-150"
      :class="state === 'running' ? 'ring-success/45' : 'ring-card-line hover:ring-line-strong'"
    >
      <InstanceIcon
        :icon="instance.icon"
        :type="instance.type"
        :version="instance.minecraftVersion"
        size="lg"
      />

      <div class="min-w-0 flex-1">
        <NuxtLink
          :to="`/instance/${instance.id}`"
          :title="instance.name"
          :class="[FOCUS_RING, 'block truncate rounded-xs text-title font-semibold text-fg after:absolute after:inset-0']"
        >
          {{ instance.name }}
        </NuxtLink>

        <p
          v-if="state === 'running'"
          class="mt-1 flex items-center gap-1.5 truncate text-caption font-medium text-success"
        >
          <KitLiveDot size="sm" />
          {{ $t('instance.state.running') }} · {{ clock }}
        </p>
        <p
          v-else-if="state === 'installing'"
          class="mt-1 truncate text-caption text-fg-muted"
        >
          {{ install?.phase ? uiText(install.phase) : $t('instance.state.installing') }}<template v-if="percent != null">
            · {{ percent }}%
          </template>
        </p>
        <p
          v-else
          class="mt-1 flex items-center gap-1.5 truncate text-caption text-fg-faint"
          :title="$t('instance.playtime.title', { playtime })"
        >
          <img
            v-if="instance.castpack"
            src="/logo.svg"
            :alt="instance.castpack.origin === 'file' ? '.cast' : 'CastPack'"
            class="size-3.25 shrink-0"
          >
          <span class="truncate">{{ loader }}</span>
          <span class="opacity-60">·</span>
          <span class="truncate">{{ playtime }}</span>
        </p>
      </div>

      <Button
        v-if="state === 'running'"
        size="round"
        icon="i-lucide-square"
        :aria-label="$t('instance.action.stop')"
        :title="$t('instance.action.stop')"
        class="z-10"
        @click="actions.stop(instance.id)"
      />
      <Button
        v-else-if="state === 'installing'"
        variant="ghost"
        size="round"
        icon="i-lucide-x"
        :disabled="install?.aborting"
        :aria-label="$t('instance.action.cancel_install')"
        :title="$t('instance.action.cancel_install')"
        class="z-10"
        @click="actions.cancelInstall(instance.id)"
      />
      <Button
        v-else-if="state === 'absent'"
        size="round"
        icon="i-lucide-arrow-down-to-line"
        :aria-label="$t('instance.action.install')"
        :title="$t('instance.action.install')"
        class="z-10"
        @click="actions.install(instance.id)"
      />
      <Button
        v-else
        variant="play"
        size="round"
        :aria-label="$t('instance.action.play')"
        :title="$t('instance.action.play')"
        class="z-10"
        @click="actions.play(instance.id)"
      >
        <Icon
          name="i-lucide-play"
          class="ml-0.5 size-4"
        />
      </Button>

      <Progress
        v-if="state === 'installing'"
        :model-value="percent"
        class="absolute inset-x-0 bottom-0 h-0.75 rounded-none"
      />
    </article>
  </InstanceActionsMenu>
</template>
