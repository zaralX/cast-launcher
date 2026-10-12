<script setup lang="ts">
import type { UiText } from '~/types/backend'
import type { CatalogPack, PackState } from '~/types/castpack'
import type { ButtonVariants } from '@/components/ui/button'
import { INSTANCE_TYPE_LABELS } from '~/types/instance'

const props = defineProps<{
  pack: CatalogPack
  state: PackState
  progress?: number
  phase?: UiText
  instanceId?: string
  icon?: string
}>()

const emit = defineEmits<{
  install: [packId: string]
  play: [instanceId: string]
}>()

const ACTIONS: Record<PackState, { labelKey: string, icon: string, variant: ButtonVariants['variant'] }> = {
  absent: { labelKey: 'castpack.action.absent', icon: 'i-lucide-arrow-down-to-line', variant: 'secondary' },
  installing: { labelKey: 'castpack.action.installing', icon: 'i-lucide-loader', variant: 'secondary' },
  outdated: { labelKey: 'castpack.action.outdated', icon: 'i-lucide-refresh-cw', variant: 'secondary' },
  ready: { labelKey: 'castpack.action.ready', icon: 'i-lucide-play', variant: 'play' },
  running: { labelKey: 'castpack.action.running', icon: 'i-lucide-activity', variant: 'secondary' },
}

const action = computed(() => ACTIONS[props.state])

const subtitle = computed(() => props.pack.summary || props.pack.description)
const details = computed(() => props.pack.summary && props.pack.description !== props.pack.summary ? props.pack.description : '')

function activate() {
  if (props.state === 'absent' || props.state === 'outdated') return emit('install', props.pack.id)
  if (props.state === 'ready' && props.instanceId) return emit('play', props.instanceId)
}
</script>

<template>
  <article class="flex flex-col rounded-md bg-panel ring-1 ring-card-line ring-inset">
    <div class="flex items-start gap-3.5 px-4 pt-4">
      <InstanceIcon
        v-if="icon"
        :icon="icon"
        :type="pack.loader ?? 'vanilla'"
        size="xl"
      />
      <KitThumb
        v-else
        :src="pack.icon"
        :alt="pack.name"
        size="xl"
      />

      <div class="min-w-0 flex-1">
        <div class="flex h-5 items-center gap-2 text-caption text-fg-faint">
          <span>CastPack</span>
          <span class="truncate font-mono text-micro">{{ pack.id }}</span>
          <Badge
            v-if="state === 'outdated'"
            variant="warning"
            class="ml-auto"
          >
            {{ $t('castpack.update') }}
          </Badge>
        </div>

        <KitTitle
          as="h3"
          size="sm"
          class="mt-1.5 truncate"
        >
          {{ pack.name }}
        </KitTitle>

        <p
          v-if="subtitle"
          class="mt-1 line-clamp-2 text-body text-fg-muted"
        >
          {{ subtitle }}
        </p>
      </div>
    </div>

    <p
      v-if="details"
      class="mx-4 mt-3 line-clamp-2 text-caption leading-relaxed text-fg-faint"
    >
      {{ details }}
    </p>

    <div class="mt-auto pt-3.5">
      <div class="flex min-h-14 items-center gap-2.5 border-t border-line py-2.5 pr-3 pl-4 text-caption text-fg-faint">
        <span
          v-if="pack.minecraftVersion"
          class="font-mono"
        >{{ pack.minecraftVersion }}</span>
        <span v-if="pack.loader">{{ INSTANCE_TYPE_LABELS[pack.loader] }}</span>
        <span
          v-if="pack.version"
          class="font-mono"
        >v{{ pack.version }}</span>

        <template v-if="pack.author">
          <Separator
            orientation="vertical"
            class="data-[orientation=vertical]:h-3"
          />
          <span class="min-w-0 truncate">{{ pack.author }}</span>
        </template>

        <span class="flex-1" />

        <div
          v-if="state === 'installing'"
          class="flex shrink-0 flex-col items-end gap-1.5"
        >
          <span class="text-fg-muted">
            {{ phase ? uiText(phase) : $t('castpack.action.installing') }}
            <span
              v-if="progress != null"
              class="font-medium text-accent tabular-nums"
            >{{ Math.round(progress * 100) }}%</span>
          </span>

          <Progress
            :model-value="progress == null ? null : progress * 100"
            class="w-36"
          />
        </div>

        <Button
          v-else
          :variant="action.variant"
          :icon="action.icon"
          class="shrink-0"
          :disabled="state === 'running'"
          @click="activate"
        >
          {{ $t(action.labelKey) }}
        </Button>
      </div>
    </div>
  </article>
</template>
