<script setup lang="ts">
import type { CastPreview } from '~/types/cast'
import type { LocalPack } from '~/types/import'
import type { Instance } from '~/types/instance'
import { PACK_PROVIDER_LABELS } from '~/types/instance'

const props = defineProps<{
  pack: LocalPack
  preview: CastPreview
  // The instance the file is about to replace, if any.
  target: Instance | null
  // Whether the player may pick between updating the existing pack and a new instance.
  choosable: boolean
}>()

const mode = defineModel<'create' | 'update'>('mode', { required: true })

const SHOWN_CODE = 6

const { t } = useI18n()

const shownCode = computed(() => props.preview.embeddedCode.slice(0, SHOWN_CODE))
const hiddenCode = computed(() => Math.max(0, props.preview.embeddedCode.length - SHOWN_CODE))

const base = computed(() => {
  const found = props.preview.base
  if (!found) return null

  return {
    name: [found.name || found.projectId, found.version].filter(Boolean).join(' '),
    provider: PACK_PROVIDER_LABELS[found.provider] ?? found.provider,
  }
})

const existing = computed(() => props.preview.existing ?? null)

const minecraftChange = computed(() => {
  const from = props.target?.minecraftVersion
  const to = props.pack.minecraftVersion

  return from && to && from !== to ? { from, to } : null
})

const otherPack = computed(() =>
  !!props.target?.castpack && props.target.castpack.catalogId !== props.preview.id)

const versionOf = (version: string) => version || t('import_pack.cast.version_unknown')
</script>

<template>
  <div class="space-y-5">
    <div class="space-y-2 text-[12px] leading-relaxed text-fg-muted">
      <p
        v-if="base"
        class="flex items-start gap-2.5"
      >
        <UIcon
          name="i-lucide-layers"
          class="mt-0.5 size-3.5 shrink-0 text-fg-faint"
        />
        {{ $t('import_pack.cast.base', base) }}
      </p>

      <p class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">
        {{ $t('import_pack.cast.sources', {
          modrinth: preview.modrinthMods,
          curseforge: preview.curseforgeMods,
          linked: preview.linkedFiles,
        }) }}
      </p>

      <p
        v-if="pack.author || preview.exportedBy"
        class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint"
      >
        <span v-if="pack.author">{{ $t('import_pack.cast.author', { author: pack.author }) }}</span>
        <span v-if="pack.author && preview.exportedBy"> · </span>
        <span v-if="preview.exportedBy">{{ $t('import_pack.cast.exported_by', { version: preview.exportedBy }) }}</span>
      </p>
    </div>

    <div
      v-if="preview.embeddedCode.length"
      class="border border-amber-400/30 bg-ink-900 px-4 py-3 text-[12px] leading-relaxed text-fg-muted"
    >
      <p class="flex items-start gap-2.5 text-fg">
        <UIcon
          name="i-lucide-shield-alert"
          class="mt-0.5 size-3.5 shrink-0 text-amber-400"
        />
        {{ $t('import_pack.cast.code_title', { count: preview.embeddedCode.length }) }}
      </p>
      <p class="mt-1.5 pl-6">
        {{ $t('import_pack.cast.code_hint') }}
      </p>
      <ul class="mt-2 space-y-0.5 pl-6 font-mono text-[10px] text-fg-faint">
        <li
          v-for="file in shownCode"
          :key="file"
          class="truncate"
          :title="file"
        >
          {{ file }}
        </li>
        <li v-if="hiddenCode">
          {{ $t('import_pack.cast.code_more', { count: hiddenCode }) }}
        </li>
      </ul>
    </div>

    <div v-if="choosable && existing">
      <span class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ $t('import_pack.cast.mode') }}</span>
      <div
        role="radiogroup"
        :aria-label="$t('import_pack.cast.mode')"
        class="grid grid-cols-2 border border-line"
      >
        <button
          v-for="(option, i) in (['update', 'create'] as const)"
          :key="option"
          type="button"
          role="radio"
          :aria-checked="mode === option"
          class="group relative flex flex-col items-start gap-1 px-4 py-3 text-left transition-colors duration-300"
          :class="[
            i > 0 ? 'border-l border-line' : '',
            mode === option ? 'bg-ink-700 text-fg' : 'text-fg-faint hover:bg-ink-700/50 hover:text-fg-muted',
          ]"
          @click="mode = option"
        >
          <span
            class="absolute inset-x-0 top-0 h-px origin-center scale-x-0 bg-acid transition-transform duration-500 ease-deck"
            :class="mode === option ? 'scale-x-100' : ''"
          />
          <span class="w-full truncate font-unbounded text-[12px] tracking-[-0.02em]">
            {{ option === 'update' ? $t('import_pack.cast.mode_update', { name: existing.name }) : $t('import_pack.cast.mode_create') }}
          </span>
          <span class="font-mono text-[10px] tracking-[0.06em] text-fg-faint">
            {{ option === 'update'
              ? $t('import_pack.cast.mode_update_hint', { from: versionOf(existing.version), to: versionOf(pack.version) })
              : $t('import_pack.cast.mode_create_hint') }}
          </span>
        </button>
      </div>
    </div>

    <p
      v-if="target && otherPack"
      class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
    >
      <UIcon
        name="i-lucide-shuffle"
        class="mt-0.5 size-3.5 shrink-0 text-amber-400"
      />
      {{ $t('import_pack.cast.other_pack', { name: target.name }) }}
    </p>

    <p
      v-if="target && minecraftChange"
      class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
    >
      <UIcon
        name="i-lucide-triangle-alert"
        class="mt-0.5 size-3.5 shrink-0 text-amber-400"
      />
      {{ $t('import_pack.cast.minecraft_changes', minecraftChange) }}
    </p>

    <div v-if="preview.changelog">
      <span class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ $t('import_pack.cast.changelog') }}</span>
      <p class="max-h-32 overflow-y-auto whitespace-pre-line border border-line px-4 py-3 text-[12px] leading-relaxed text-fg-muted">
        {{ preview.changelog }}
      </p>
    </div>
  </div>
</template>
