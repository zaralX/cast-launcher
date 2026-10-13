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
const MODES = ['update', 'create'] as const

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
  <div class="space-y-4">
    <KitNote
      :icon="base ? 'i-lucide-layers' : 'i-lucide-package'"
      tone="muted"
    >
      <template v-if="base">
        {{ $t('import_pack.cast.base', base) }} ·
      </template>
      {{ $t('import_pack.cast.sources', {
        modrinth: preview.modrinthMods,
        curseforge: preview.curseforgeMods,
        linked: preview.linkedFiles,
      }) }}
    </KitNote>

    <Alert
      v-if="preview.embeddedCode.length"
      variant="warning"
      icon="i-lucide-shield-alert"
    >
      <AlertTitle>
        {{ $t('import_pack.cast.code_title', { count: preview.embeddedCode.length }) }}
      </AlertTitle>
      <AlertDescription class="not-first:mt-1">
        {{ $t('import_pack.cast.code_hint') }}
      </AlertDescription>
      <ul class="mt-2 space-y-0.5 font-mono text-caption text-fg-faint">
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
    </Alert>

    <KitField
      v-if="choosable && existing"
      :label="$t('import_pack.cast.mode')"
    >
      <RadioGroup
        v-model="mode"
        variant="cards"
        :aria-label="$t('import_pack.cast.mode')"
        class="grid-cols-2"
      >
        <RadioGroupItem
          v-for="option in MODES"
          :key="option"
          :value="option"
          class="min-w-0 flex-col items-start gap-0.5 px-3.5 py-3"
        >
          <span class="w-full truncate font-medium text-fg">
            {{ option === 'update' ? $t('import_pack.cast.mode_update', { name: existing.name }) : $t('import_pack.cast.mode_create') }}
          </span>
          <span class="w-full truncate text-caption text-fg-faint">
            {{ option === 'update'
              ? $t('import_pack.cast.mode_update_hint', { from: versionOf(existing.version), to: versionOf(pack.version) })
              : $t('import_pack.cast.mode_create_hint') }}
          </span>
        </RadioGroupItem>
      </RadioGroup>
    </KitField>

    <KitNote
      v-if="target && otherPack"
      icon="i-lucide-shuffle"
    >
      {{ $t('import_pack.cast.other_pack', { name: target.name }) }}
    </KitNote>

    <KitNote
      v-if="target && minecraftChange"
      icon="i-lucide-triangle-alert"
    >
      {{ $t('import_pack.cast.minecraft_changes', minecraftChange) }}
    </KitNote>

    <KitField
      v-if="preview.changelog"
      :label="$t('import_pack.cast.changelog')"
    >
      <p class="max-h-32 overflow-y-auto rounded-md bg-panel px-3.5 py-2.5 text-body whitespace-pre-line text-fg-muted ring-1 ring-card-line ring-inset">
        {{ preview.changelog }}
      </p>
    </KitField>
  </div>
</template>
