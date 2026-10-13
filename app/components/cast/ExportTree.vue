<script setup lang="ts">
import type { SourceKind, TreeEntry } from '~/types/cast'
import { EXPORT_NOTE_KEYS, SOURCE_KEYS } from '~/types/cast'

defineProps<{ tree: TreeEntry[] }>()

const selected = defineModel<Set<string>>('selected', { required: true })

const { t } = useI18n()

const expanded = ref(new Set<string>())

const choosable = (entry: TreeEntry) => entry.note !== 'forbidden'

// What a checkbox of the entry stands for: its children when it has them, itself otherwise.
const leaves = (entry: TreeEntry) =>
  (entry.children?.length ? entry.children : [entry]).filter(choosable)

function state(entry: TreeEntry): boolean | 'indeterminate' {
  const items = leaves(entry)
  const picked = items.filter(item => selected.value.has(item.key)).length

  if (!picked) return false
  return picked === items.length ? true : 'indeterminate'
}

function toggle(entry: TreeEntry, value: boolean | 'indeterminate') {
  const next = new Set(selected.value)

  for (const item of leaves(entry)) {
    if (value === true) next.add(item.key)
    else next.delete(item.key)
  }

  selected.value = next
}

function toggleExpanded(key: string) {
  const next = new Set(expanded.value)

  if (next.has(key)) next.delete(key)
  else next.add(key)

  expanded.value = next
}

const SOURCE_TONE: Record<SourceKind, string> = {
  modrinth: 'text-accent',
  curseforge: 'text-accent',
  embedded: 'text-warning',
  unchecked: 'text-warning',
}

function sources(entry: TreeEntry) {
  const counts: Partial<Record<SourceKind, number>> = {}
  let inBase = 0

  for (const child of entry.children ?? []) {
    if (!selected.value.has(child.key)) continue
    if (child.inBase) inBase++
    if (!child.source) continue
    counts[child.source.kind] = (counts[child.source.kind] ?? 0) + 1
  }

  const found = (Object.keys(SOURCE_KEYS) as SourceKind[])
    .filter(kind => counts[kind])
    .map(kind => ({ label: t(SOURCE_KEYS[kind]), tone: SOURCE_TONE[kind], count: counts[kind]! }))

  if (inBase) found.push({ label: t('cast_export.in_base'), tone: 'text-fg-faint', count: inBase })

  return found
}

const sizeOf = (entry: TreeEntry) => entry.dir
  ? `${formatBytes(entry.size)} · ${t('cast_export.files', { count: entry.files })}`
  : formatBytes(entry.size)
</script>

<template>
  <ul class="divide-y divide-line overflow-hidden rounded-md bg-panel ring-1 ring-card-line ring-inset">
    <li
      v-for="entry in tree"
      :key="entry.key"
    >
      <div class="flex items-center gap-2.5 px-3 py-2.5">
        <Button
          v-if="entry.children?.length"
          variant="ghost"
          size="icon-sm"
          class="-my-1 size-6"
          :aria-label="$t('cast_export.expand')"
          :aria-expanded="expanded.has(entry.key)"
          @click="toggleExpanded(entry.key)"
        >
          <Icon
            name="i-lucide-chevron-right"
            class="size-3.5 transition-transform duration-150"
            :class="expanded.has(entry.key) && 'rotate-90'"
          />
        </Button>
        <span
          v-else
          class="size-6 shrink-0"
        />

        <Checkbox
          :model-value="state(entry)"
          :disabled="!leaves(entry).length"
          @update:model-value="value => toggle(entry, value)"
        />

        <Icon
          :name="entry.dir ? 'i-lucide-folder' : 'i-lucide-file'"
          class="size-4 shrink-0 text-fg-faint"
        />

        <div class="min-w-0 flex-1">
          <p
            class="truncate text-fg"
            :title="entry.key"
          >
            {{ entry.name }}
          </p>
          <p
            v-if="sources(entry).length"
            class="truncate text-caption text-fg-faint"
          >
            <template
              v-for="(source, i) in sources(entry)"
              :key="source.label"
            >
              <span v-if="i"> · </span>
              <span :class="source.tone">{{ source.label }} {{ source.count }}</span>
            </template>
          </p>
        </div>

        <span
          v-if="entry.inBase"
          class="shrink-0 text-caption text-fg-faint"
        >
          {{ $t('cast_export.in_base') }}
        </span>
        <Badge v-if="entry.note">
          {{ $t(EXPORT_NOTE_KEYS[entry.note]) }}
        </Badge>
        <span class="w-36 shrink-0 text-right font-mono text-caption text-fg-faint tabular-nums">
          {{ sizeOf(entry) }}
        </span>
      </div>

      <ul
        v-if="entry.children?.length && expanded.has(entry.key)"
        class="max-h-72 overflow-y-auto border-t border-line bg-sheet/40"
      >
        <li
          v-for="child in entry.children"
          :key="child.key"
          class="flex items-center gap-2.5 py-1.5 pr-3 pl-[4.25rem]"
        >
          <Checkbox
            :model-value="selected.has(child.key)"
            :disabled="!choosable(child)"
            @update:model-value="value => toggle(child, value)"
          />
          <span
            class="min-w-0 flex-1 truncate text-fg-muted"
            :title="child.key"
          >
            {{ child.name }}
          </span>
          <span
            v-if="child.inBase"
            class="shrink-0 text-caption text-fg-faint"
          >
            {{ $t('cast_export.in_base') }}
          </span>
          <span
            v-if="child.source"
            class="max-w-48 shrink-0 truncate text-caption"
            :class="SOURCE_TONE[child.source.kind]"
            :title="child.source.title"
          >
            {{ child.source.title || $t(SOURCE_KEYS[child.source.kind]) }}
          </span>
          <Badge v-if="child.note">
            {{ $t(EXPORT_NOTE_KEYS[child.note]) }}
          </Badge>
          <span class="w-36 shrink-0 text-right font-mono text-caption text-fg-faint tabular-nums">
            {{ sizeOf(child) }}
          </span>
        </li>
      </ul>
    </li>
  </ul>
</template>
