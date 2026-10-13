<script setup lang="ts">
import type { LocalPack } from '~/types/import'
import { CAST_EXTENSION } from '~/types/cast'

const props = defineProps<{
  // A file to read right away, for example one opened from the system.
  path?: string
  // The instance a .cast file updates, when the dialog was opened from it.
  updateTarget?: string
}>()

const emit = defineEmits<{ imported: [instanceId: string, updated: boolean] }>()

const { t } = useI18n()
const instanceStore = useInstanceStore()

const picking = ref(false)
const reading = ref(false)
const importing = ref(false)

const pack = ref<LocalPack | null>(null)

const name = ref('')
const description = ref('')
const mode = ref<'create' | 'update'>('create')

const busy = computed(() => picking.value || reading.value || importing.value)

const targetId = computed(() => {
  if (props.updateTarget) return props.updateTarget
  if (mode.value === 'update') return pack.value?.cast?.existing?.instanceId ?? null
  return null
})

const target = computed(() => targetId.value ? instanceStore.getInstance(targetId.value) ?? null : null)
const updating = computed(() => !!targetId.value)

// An update needs a .cast file: other formats only make new instances.
const wrongFile = computed(() => !!props.updateTarget && !!pack.value && !pack.value.cast)

const canImport = computed(() =>
  !!pack.value
  && !pack.value.blocked
  && !wrongFile.value
  && !busy.value
  && (updating.value || name.value.trim().length > 0),
)

const facts = computed(() => {
  const found = pack.value
  if (!found) return []

  const cast = found.cast

  if (cast) {
    const fromBase = cast.base ? t('import_pack.cast.from_base') : '-'

    return [
      { label: t('import_pack.facts.format'), value: found.kindLabel },
      { label: 'Minecraft', value: found.minecraftVersion || fromBase },
      { label: t('import_pack.facts.loader'), value: found.loader ? found.loaderLabel : fromBase },
      { label: t('import_pack.facts.version'), value: found.version || '-' },
      { label: t('import_pack.cast.mods'), value: String(cast.modrinthMods + cast.curseforgeMods) },
      {
        label: t('import_pack.cast.inside'),
        value: cast.embeddedFiles
          ? `${cast.embeddedFiles} · ${formatBytes(cast.embeddedSize)}`
          : t('import_pack.cast.inside_none'),
      },
    ]
  }

  return [
    { label: t('import_pack.facts.format'), value: found.kindLabel },
    { label: 'Minecraft', value: found.minecraftVersion || '-' },
    { label: t('import_pack.facts.loader'), value: found.loader ? found.loaderLabel : '-' },
    { label: t('import_pack.facts.version'), value: found.version || '-' },
    { label: t('import_pack.facts.files'), value: found.files ? String(found.files) : t('import_pack.facts.files_inside') },
    { label: t('import_pack.facts.size'), value: formatBytes(found.size) },
  ]
})

async function choose() {
  if (busy.value) return

  picking.value = true
  const path = await safeRun(
    () => call('pick_modpack_file', { dialog: { title: t('dialog.modpack.title'), filter: t('dialog.modpack.filter') } }),
    { context: { action: t('import_pack.pick_action') } },
  )
  picking.value = false

  if (path) await read(path)
}

async function read(path: string) {
  reading.value = true
  pack.value = null

  const result = await attempt(() => call('inspect_modpack_file', { path }), {
    context: { action: t('import_pack.read_action') },
  })

  reading.value = false

  if (!result.ok) return

  pack.value = result.value
  name.value = result.value.name
  description.value = result.value.description
  mode.value = result.value.cast?.existing ? 'update' : 'create'
}

onMounted(() => {
  if (props.path) read(props.path)
})

const EXTENSIONS = ['.mrpack', '.zip', CAST_EXTENSION]

function isPack(path: string): boolean {
  return EXTENSIONS.some(extension => path.toLowerCase().endsWith(extension))
}

const dragging = useFileDrop((paths) => {
  if (busy.value) return

  const dropped = paths.find(isPack)
  if (dropped) read(dropped)
})

async function run() {
  if (!canImport.value || !pack.value) return

  importing.value = true

  const updated = updating.value

  const created = await attempt(() => call('import_modpack_file', {
    request: {
      path: pack.value!.path,
      name: updated ? undefined : name.value.trim(),
      description: updated ? undefined : description.value.trim(),
      update: targetId.value ?? undefined,
    },
  }), { context: { action: t(updated ? 'import_pack.cast.update_action' : 'import_pack.import_action') } })

  if (!created.ok) {
    importing.value = false
    return
  }

  await safeRun(() => instanceStore.installInstance(created.value.id), {
    code: 'NETWORK',
    context: { action: t('import_pack.install_action'), instanceName: created.value.name },
  })

  importing.value = false
  emit('imported', created.value.id, updated)
}
</script>

<template>
  <div class="space-y-5">
    <div
      v-if="pack"
      class="flex items-center gap-3.5 rounded-md px-3.5 py-3 ring-[1.5px] ring-inset transition-colors duration-150"
      :class="dragging && !busy ? 'bg-accent/6 ring-accent' : 'ring-line-strong'"
    >
      <span class="grid size-10 shrink-0 place-items-center rounded-sm bg-control text-accent">
        <Spinner
          v-if="reading"
          class="size-4.5"
        />
        <Icon
          v-else
          name="i-lucide-file-archive"
          class="size-4.5"
        />
      </span>
      <div class="min-w-0 flex-1">
        <p
          class="truncate font-medium text-fg"
          :title="pack.path"
        >
          {{ pack.fileName }}
        </p>
        <p class="truncate text-caption text-fg-faint">
          {{ [pack.kindLabel, pack.author].filter(Boolean).join(' · ') }}
        </p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        :disabled="busy"
        @click="choose"
      >
        {{ $t('import_pack.pick_another') }}
      </Button>
    </div>

    <Button
      v-else
      variant="dashed"
      class="h-auto w-full flex-col gap-2 rounded-md px-6 py-10 font-normal whitespace-normal"
      :class="dragging && !busy && 'border-accent bg-accent/6 text-accent'"
      :disabled="busy"
      @click="choose"
    >
      <Spinner
        v-if="reading"
        class="size-5"
      />
      <Icon
        v-else
        name="i-lucide-file-archive"
        class="size-5"
      />
      <span class="font-medium">
        {{ reading ? $t('import_pack.reading') : $t('import_pack.pick') }}
      </span>
      <span class="text-caption text-fg-faint">
        {{ $t('import_pack.hint_before') }}
        <span class="text-fg-muted">{{ updateTarget ? CAST_EXTENSION : '.mrpack' }}</span>
        {{ updateTarget ? $t('import_pack.cast.update_hint') : $t('import_pack.hint_after') }}
      </span>
    </Button>

    <Alert
      v-if="pack?.blocked"
      variant="warning"
      icon="i-lucide-triangle-alert"
    >
      {{ $t('import_pack.blocked', { reason: uiText(pack.blocked) }) }}
    </Alert>

    <Alert
      v-else-if="wrongFile"
      variant="warning"
      icon="i-lucide-triangle-alert"
    >
      {{ $t('import_pack.cast.not_cast') }}
    </Alert>

    <form
      v-if="pack"
      class="space-y-5"
      @submit.prevent="run"
    >
      <template v-if="!updating">
        <KitField
          :label="$t('import_pack.name')"
          for="file-pack-name"
        >
          <Input
            id="file-pack-name"
            v-model="name"
            size="lg"
          />
        </KitField>
        <KitField
          :label="$t('import_pack.description')"
          for="file-pack-description"
        >
          <Input
            id="file-pack-description"
            v-model="description"
            :placeholder="$t('import_pack.description_placeholder')"
          />
        </KitField>
      </template>

      <KitStatGrid :columns="3">
        <KitStat
          v-for="fact in facts"
          :key="fact.label"
          :label="fact.label"
          :value="fact.value"
        />
      </KitStatGrid>

      <ImportCastDetails
        v-if="pack.cast"
        v-model:mode="mode"
        :pack="pack"
        :preview="pack.cast"
        :target="target"
        :choosable="!updateTarget"
      />

      <KitNote
        v-if="pack.kind === 'curseforge' || (pack.cast?.curseforgeMods ?? 0) > 0"
        icon="i-lucide-hand"
      >
        {{ $t('import_pack.curseforge_hint') }}
      </KitNote>

      <div class="flex justify-end">
        <Button
          variant="primary"
          type="submit"
          icon="i-lucide-download"
          :loading="importing"
          :disabled="!canImport"
        >
          <template v-if="updating">
            {{ importing ? $t('import_pack.cast.updating') : $t('import_pack.cast.update') }}
          </template>
          <template v-else>
            {{ importing ? $t('import_pack.creating') : $t('import_pack.import') }}
          </template>
        </Button>
      </div>
    </form>
  </div>
</template>
