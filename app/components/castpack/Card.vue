<script setup lang="ts">
import type { UiText } from '~/types/backend'
import type { CatalogPack, PackState } from '~/types/castpack'
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

const ACTIONS: Record<PackState, { labelKey: string, icon: string }> = {
  absent: { labelKey: 'castpack.action.absent', icon: 'i-lucide-arrow-down-to-line' },
  installing: { labelKey: 'castpack.action.installing', icon: 'i-lucide-loader' },
  outdated: { labelKey: 'castpack.action.outdated', icon: 'i-lucide-refresh-cw' },
  ready: { labelKey: 'castpack.action.ready', icon: 'i-lucide-play' },
  running: { labelKey: 'castpack.action.running', icon: 'i-lucide-activity' },
}

const action = computed(() => ACTIONS[props.state])

const meta = computed(() => [
  props.pack.minecraftVersion,
  props.pack.loader ? INSTANCE_TYPE_LABELS[props.pack.loader] : '',
  props.pack.version ? `v${props.pack.version}` : '',
].filter(Boolean))

function activate() {
  if (props.state === 'absent' || props.state === 'outdated') return emit('install', props.pack.id)
  if (props.state === 'ready' && props.instanceId) return emit('play', props.instanceId)
}
</script>

<template>
  <article class="group relative flex flex-col justify-between overflow-hidden border border-line bg-ink-800 transition-all duration-500 ease-deck hover:border-acid/40 hover:bg-ink-700">
    <span
      class="absolute inset-y-0 left-0 w-[2px] origin-top scale-y-0 bg-acid transition-transform duration-700 ease-deck group-hover:scale-y-100"
      aria-hidden="true"
    />

    <div class="p-4 pl-5">
      <div class="flex items-center gap-3">
        <span class="font-mono text-micro uppercase tracking-caps-wide text-acid">CastPack</span>
        <span class="h-px w-6 bg-acid/40" />
        <span class="truncate font-mono text-micro uppercase tracking-caps text-fg-faint">{{ pack.id }}</span>

        <Badge
          v-if="state === 'outdated'"
          variant="warning"
          class="ml-auto"
        >
          {{ $t('castpack.update') }}
        </Badge>
      </div>

      <div class="mt-3 flex items-start gap-3.5">
        <InstanceIcon
          v-if="icon"
          :icon="icon"
          :type="pack.loader ?? 'vanilla'"
          class="shrink-0 text-fg-faint transition-colors duration-500 group-hover:border-acid/40 group-hover:text-acid"
        />

        <span
          v-else-if="pack.icon"
          class="grid size-8 shrink-0 place-items-center overflow-hidden border border-line transition-colors duration-500 group-hover:border-acid/40"
        >
          <img
            :src="pack.icon"
            alt=""
            class="size-full object-contain p-[3px]"
          >
        </span>

        <div class="min-w-0">
          <h3 class="font-unbounded text-[16px] font-semibold leading-[1.1] tracking-heading text-fg">
            {{ pack.name }}
          </h3>
          <p class="mt-2 line-clamp-2 max-w-[46ch] text-body leading-relaxed text-fg-muted">
            {{ pack.summary || pack.description }}
          </p>
        </div>
      </div>
    </div>

    <div class="flex items-center justify-between gap-3 border-t border-line px-4 py-2 pl-5">
      <div class="flex min-w-0 items-center gap-3 font-mono text-micro uppercase tracking-caps text-fg-faint">
        <template
          v-for="(item, i) in meta"
          :key="item"
        >
          <Separator
            v-if="i"
            orientation="vertical"
            class="h-3"
          />
          <span class="truncate">{{ item }}</span>
        </template>
      </div>

      <div
        v-if="state === 'installing'"
        class="flex shrink-0 flex-col items-end gap-1.5"
      >
        <span class="font-mono text-micro uppercase tracking-caps text-fg-muted">
          {{ phase ? uiText(phase) : $t('castpack.action.installing') }}
          <span
            v-if="progress != null"
            class="text-acid"
          >{{ Math.round(progress * 100) }}%</span>
        </span>

        <Progress
          :model-value="progress == null ? null : progress * 100"
          class="w-24"
        />
      </div>

      <Button
        v-else
        variant="quiet"
        class="group/act shrink-0"
        :disabled="state === 'running'"
        @click="activate"
      >
        {{ $t(action.labelKey) }}
        <Icon
          :name="action.icon"
          class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-1"
        />
      </Button>
    </div>
  </article>
</template>
