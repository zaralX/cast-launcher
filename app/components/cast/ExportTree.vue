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
  modrinth: 'text-acid',
  curseforge: 'text-acid',
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
  <ul class="divide-y divide-line border border-line">
    <li
      v-for="entry in tree"
      :key="entry.key"
    >
      <div class="flex items-center gap-3 px-3 py-2.5">
        <button
          v-if="entry.children?.length"
          type="button"
          class="grid size-5 shrink-0 cursor-pointer place-items-center text-fg-faint outline-none transition-colors duration-300 hover:text-fg focus-visible:text-fg"
          :aria-label="$t('cast_export.expand')"
          :aria-expanded="expanded.has(entry.key)"
          @click="toggleExpanded(entry.key)"
        >
          <Icon
            name="i-lucide-chevron-right"
            class="size-3.5 transition-transform duration-300"
            :class="expanded.has(entry.key) ? 'rotate-90' : ''"
          />
        </button>
        <span
          v-else
          class="size-5 shrink-0"
        />

        <Checkbox
          :model-value="state(entry)"
          :disabled="!leaves(entry).length"
          @update:model-value="value => toggle(entry, value)"
        />

        <Icon
          :name="entry.dir ? 'i-lucide-folder' : 'i-lucide-file'"
          class="size-3.5 shrink-0 text-fg-faint"
        />

        <div class="min-w-0 flex-1">
          <p
            class="truncate text-body text-fg"
            :title="entry.key"
          >
            {{ entry.name }}
          </p>
          <p
            v-if="sources(entry).length"
            class="mt-0.5 truncate font-mono text-micro uppercase tracking-caps text-fg-faint"
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
          class="shrink-0 font-mono text-micro uppercase tracking-caps text-fg-faint"
        >
          {{ $t('cast_export.in_base') }}
        </span>

        <Badge
          v-if="entry.note"
          class="text-fg-faint"
        >
          {{ $t(EXPORT_NOTE_KEYS[entry.note]) }}
        </Badge>

        <span class="w-36 shrink-0 text-right font-mono text-label tabular-nums text-fg-faint">
          {{ sizeOf(entry) }}
        </span>
      </div>

      <ul
        v-if="entry.children?.length && expanded.has(entry.key)"
        class="max-h-72 overflow-y-auto border-t border-line bg-ink-900/40"
      >
        <li
          v-for="child in entry.children"
          :key="child.key"
          class="flex items-center gap-3 py-1.5 pr-3 pl-14"
        >
          <Checkbox
            :model-value="selected.has(child.key)"
            :disabled="!choosable(child)"
            @update:model-value="value => toggle(child, value)"
          />

          <span
            class="min-w-0 flex-1 truncate text-body text-fg-muted"
            :title="child.key"
          >
            {{ child.name }}
          </span>

          <span
            v-if="child.inBase"
            class="shrink-0 font-mono text-micro uppercase tracking-caps text-fg-faint"
          >
            {{ $t('cast_export.in_base') }}
          </span>

          <span
            v-if="child.source"
            class="max-w-48 shrink-0 truncate font-mono text-micro uppercase tracking-caps"
            :class="SOURCE_TONE[child.source.kind]"
            :title="child.source.title"
          >
            {{ child.source.title || $t(SOURCE_KEYS[child.source.kind]) }}
          </span>

          <span
            v-if="child.note"
            class="shrink-0 font-mono text-micro uppercase tracking-caps text-fg-faint"
          >
            {{ $t(EXPORT_NOTE_KEYS[child.note]) }}
          </span>

          <span class="w-36 shrink-0 text-right font-mono text-label tabular-nums text-fg-faint">
            {{ sizeOf(child) }}
          </span>
        </li>
      </ul>
    </li>
  </ul>
</template>
