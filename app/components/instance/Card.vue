<script setup lang="ts">
import type { UiText } from '~/types/backend'
import type { Instance } from '~/types/instance'

const props = defineProps<{
  instance: Instance
  running?: boolean
  installing?: boolean
  progress?: number
  phase?: UiText
}>()

const emit = defineEmits<{
  install: [id: string]
  run: [id: string]
}>()

const state = computed<'running' | 'installing' | 'ready' | 'absent'>(() => {
  if (props.running) return 'running'
  if (props.installing) return 'installing'
  return props.instance.installed ? 'ready' : 'absent'
})

const { t } = useI18n()

const { total } = usePlaytime(() => props.instance)

const playtime = computed(() => formatPlaytime(total.value) || t('instance.playtime.never'))
</script>

<template>
  <article
    class="cut-16 group relative flex flex-col border border-line bg-ink-800 p-3.5 transition-all duration-500 ease-deck hover:-translate-y-0.5 hover:border-acid/40 hover:bg-ink-700"
  >
    <span
      class="pointer-events-none absolute top-0 right-0 h-[23px] w-px origin-top-right rotate-45 bg-line transition-colors duration-500 group-hover:bg-acid/40"
      aria-hidden="true"
    />

    <header class="flex min-w-0 items-center gap-3">
      <InstanceIcon
        :icon="instance.icon"
        :type="instance.type"
        class="text-fg-faint transition-colors duration-500 group-hover:border-acid/40 group-hover:text-acid"
      />

      <div class="min-w-0 flex-1 pr-3">
        <h3
          class="truncate font-unbounded text-title font-semibold leading-tight tracking-heading text-fg"
          :title="instance.name"
        >
          {{ instance.name }}
        </h3>
        <p
          v-if="!instance.castpack"
          class="mt-1 truncate font-mono text-micro uppercase tracking-caps text-fg-faint"
        >
          {{ instance.minecraftVersion }}
          <template v-if="instance.loaderVersion">
            · {{ instance.loaderVersion }}
          </template>
        </p>
        <p
          v-else
          class="mt-1 truncate font-mono text-micro uppercase tracking-caps text-fg-faint"
          :title="$t(instance.castpack.origin === 'file' ? 'instance.cast_file_tooltip' : 'instance.castpack_tooltip', { version: instance.castpack.version })"
        >
          {{ instance.castpack.origin === 'file' ? '.cast' : 'CastPack' }}{{ instance.castpack.version ? ` ${instance.castpack.version}` : '' }}
        </p>
        <p
          class="mt-1 flex items-center gap-1.5 truncate font-mono text-micro uppercase tracking-caps text-fg-faint"
          :title="$t('instance.playtime.title', { playtime })"
        >
          <Icon
            name="i-lucide-timer"
            class="size-2.5 shrink-0"
          />
          <span class="truncate">{{ playtime }}</span>
        </p>
      </div>
    </header>

    <Separator class="mt-3 transition-colors duration-500 group-hover:bg-line-strong" />

    <footer class="mt-2.5 flex items-center gap-2">
      <div class="min-w-0 flex-1">
        <KitStatus
          v-if="state === 'running'"
          tone="accent"
          dot="live"
          class="h-8 items-center px-1"
        >
          {{ $t('instance.state.running') }}
        </KitStatus>

        <InstanceInstallProgress
          v-else-if="state === 'installing'"
          :phase="phase"
          :progress="progress"
          class="h-8 px-1"
        />

        <Button
          v-else
          size="sm"
          class="group/act w-full"
          @click="state === 'ready' ? emit('run', instance.id) : emit('install', instance.id)"
        >
          <Icon
            :name="state === 'ready' ? 'i-lucide-play' : 'i-lucide-arrow-down-to-line'"
            class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-0.5"
          />
          {{ state === 'ready' ? $t('instance.action.play') : $t('instance.action.install') }}
        </Button>
      </div>

      <Button
        :to="`/instance/${instance.id}`"
        variant="outline"
        size="icon-sm"
        :aria-label="$t('instance.settings_title', { name: instance.name })"
        :title="$t('instance.settings_title', { name: instance.name })"
        class="group/cfg shrink-0 text-fg-faint hover:border-acid hover:text-acid"
      >
        <Icon
          name="i-lucide-settings"
          class="size-3.5 transition-transform duration-500 ease-deck group-hover/cfg:rotate-90"
        />
      </Button>
    </footer>
  </article>
</template>
