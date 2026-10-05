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
  <div>
    <button
      type="button"
      class="group flex w-full flex-col items-center gap-3 border border-dashed px-6 py-10 transition-colors duration-300"
      :class="dragging && !busy ? 'border-acid/60 bg-acid/[0.04] text-acid' : 'border-line text-fg-faint hover:border-acid/50 hover:text-acid'"
      :disabled="busy"
      @click="choose"
    >
      <UIcon
        :name="reading ? 'i-lucide-loader' : 'i-lucide-file-archive'"
        class="size-5 transition-transform duration-500 ease-deck group-hover:-translate-y-0.5"
        :class="reading ? 'animate-spin' : ''"
      />
      <span class="font-mono text-[10px] uppercase tracking-[0.24em]">
        {{ reading ? $t('import_pack.reading') : pack ? $t('import_pack.pick_another') : $t('import_pack.pick') }}
      </span>
      <span
        v-if="updateTarget"
        class="text-[12px] leading-relaxed text-fg-muted"
      >
        {{ $t('import_pack.hint_before') }} <span class="text-fg">{{ CAST_EXTENSION }}</span> {{ $t('import_pack.cast.update_hint') }}
      </span>
      <span
        v-else
        class="text-[12px] leading-relaxed text-fg-muted"
      >
        {{ $t('import_pack.hint_before') }} <span class="text-fg">.mrpack</span> {{ $t('import_pack.hint_after') }}
      </span>
    </button>

    <p
      v-if="pack"
      class="mt-3 truncate font-mono text-[10px] text-fg-faint"
      :title="pack.path"
    >
      {{ pack.fileName }}
    </p>

    <div
      v-if="pack?.blocked"
      class="mt-6 flex items-start gap-2.5 border border-amber-400/30 bg-ink-900 px-4 py-3 text-[12px] leading-relaxed text-fg-muted"
    >
      <UIcon
        name="i-lucide-triangle-alert"
        class="mt-0.5 size-3.5 shrink-0 text-amber-400"
      />
      {{ $t('import_pack.blocked', { reason: uiText(pack.blocked) }) }}
    </div>

    <div
      v-else-if="wrongFile"
      class="mt-6 flex items-start gap-2.5 border border-amber-400/30 bg-ink-900 px-4 py-3 text-[12px] leading-relaxed text-fg-muted"
    >
      <UIcon
        name="i-lucide-triangle-alert"
        class="mt-0.5 size-3.5 shrink-0 text-amber-400"
      />
      {{ $t('import_pack.cast.not_cast') }}
    </div>

    <form
      v-if="pack"
      class="mt-6 space-y-8"
      @submit.prevent="run"
    >
      <div
        v-if="!updating"
        class="space-y-5"
      >
        <div>
          <label
            for="file-pack-name"
            class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            {{ $t('import_pack.name') }}
          </label>
          <UInput
            id="file-pack-name"
            v-model="name"
            size="lg"
            class="w-full"
            :ui="{ base: 'font-unbounded text-[15px] tracking-[-0.03em]' }"
          />
        </div>

        <div>
          <label
            for="file-pack-description"
            class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            {{ $t('import_pack.description') }}
          </label>
          <UInput
            id="file-pack-description"
            v-model="description"
            :placeholder="$t('import_pack.description_placeholder')"
            class="w-full"
          />
        </div>
      </div>

      <dl class="grid grid-cols-2 border border-line sm:grid-cols-3">
        <div
          v-for="(fact, i) in facts"
          :key="fact.label"
          class="px-4 py-3"
          :class="i % 3 === 0 ? '' : 'sm:border-l sm:border-line'"
        >
          <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
            {{ fact.label }}
          </dt>
          <dd
            class="mt-1.5 truncate font-unbounded text-[13px] tracking-[-0.03em] text-fg"
            :title="fact.value"
          >
            {{ fact.value }}
          </dd>
        </div>
      </dl>

      <ImportCastDetails
        v-if="pack.cast"
        v-model:mode="mode"
        :pack="pack"
        :preview="pack.cast"
        :target="target"
        :choosable="!updateTarget"
      />

      <p
        v-if="pack.kind === 'curseforge' || (pack.cast?.curseforgeMods ?? 0) > 0"
        class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
      >
        <UIcon
          name="i-lucide-hand"
          class="mt-0.5 size-3.5 shrink-0 text-amber-400"
        />
        {{ $t('import_pack.curseforge_hint') }}
      </p>

      <p
        v-else-if="pack.kind === 'multimc'"
        class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
      >
        <UIcon
          name="i-lucide-info"
          class="mt-0.5 size-3.5 shrink-0 text-fg-faint"
        />
        {{ $t('import_pack.multimc_hint') }}
      </p>

      <AppButton
        block
        type="submit"
        class="group/act h-11 tracking-[0.2em]"
        :loading="importing"
        :disabled="!canImport"
      >
        <template #leading>
          <UIcon
            name="i-lucide-download"
            class="size-3.5 transition-transform duration-500 group-hover/act:translate-y-0.5"
          />
        </template>
        <template v-if="updating">
          {{ importing ? $t('import_pack.cast.updating') : $t('import_pack.cast.update') }}
        </template>
        <template v-else>
          {{ importing ? $t('import_pack.creating') : $t('import_pack.import') }}
        </template>
      </AppButton>
    </form>
  </div>
</template>
