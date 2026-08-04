<script setup lang="ts">
import {getCurrentWebview} from "@tauri-apps/api/webview";
import type {UnlistenFn} from "@tauri-apps/api/event";
import {call} from "~/types/backend";
import type {LocalPack} from "~/types/import";
import {formatBytes} from "~/types/import";
import {INSTANCE_TYPE_LABELS} from "~/types/instance";

const emit = defineEmits<{ imported: [instanceId: string] }>()

const instanceStore = useInstanceStore()

const picking = ref(false)
const reading = ref(false)
const importing = ref(false)
const dragging = ref(false)

const pack = ref<LocalPack | null>(null)

const name = ref("")
const description = ref("")

const busy = computed(() => picking.value || reading.value || importing.value)

const canImport = computed(() =>
    !!pack.value && !pack.value.blocked && !busy.value && name.value.trim().length > 0
)

const facts = computed(() => {
  const found = pack.value
  if (!found) return []

  return [
    {label: "Формат", value: found.kindLabel},
    {label: "Minecraft", value: found.minecraftVersion || "-"},
    {label: "Загрузчик", value: found.loader ? found.loaderLabel : "-"},
    {label: "Версия пака", value: found.version || "-"},
    {label: "Файлов", value: found.files ? String(found.files) : "внутри архива"},
    {label: "Размер", value: formatBytes(found.size)}
  ]
})

async function choose() {
  if (busy.value) return

  picking.value = true
  const path = await safeRun(() => call("pick_modpack_file"), {context: {action: "Выбор файла модпака"}})
  picking.value = false

  if (path) await read(path)
}

async function read(path: string) {
  reading.value = true
  pack.value = null

  const result = await attempt(() => call("inspect_modpack_file", {path}), {
    context: {action: "Чтение файла модпака"}
  })

  reading.value = false

  if (!result.ok) return

  pack.value = result.value
  name.value = result.value.name
  description.value = result.value.description
}

const EXTENSIONS = [".mrpack", ".zip"]

function isPack(path: string): boolean {
  return EXTENSIONS.some(extension => path.toLowerCase().endsWith(extension))
}

let unlisten: UnlistenFn | undefined

onMounted(async () => {
  unlisten = await safeRun(() => getCurrentWebview().onDragDropEvent(event => {
    if (event.payload.type === "enter" || event.payload.type === "over") {
      dragging.value = !busy.value
      return
    }

    dragging.value = false

    if (event.payload.type !== "drop" || busy.value) return

    const dropped = event.payload.paths.find(isPack)
    if (dropped) read(dropped)
  }))
})

onBeforeUnmount(() => unlisten?.())

async function run() {
  if (!canImport.value || !pack.value) return

  importing.value = true

  const created = await attempt(() => call("import_modpack_file", {
    request: {
      path: pack.value!.path,
      name: name.value.trim(),
      description: description.value.trim()
    }
  }), {context: {action: "Импорт модпака из файла"}})

  if (!created.ok) {
    importing.value = false
    return
  }

  await safeRun(() => instanceStore.installInstance(created.value.id), {
    code: "NETWORK",
    context: {action: "Установка модпака", instanceName: created.value.name}
  })

  importing.value = false
  emit("imported", created.value.id)
}
</script>

<template>
  <div>
    <button
        type="button"
        class="group flex w-full flex-col items-center gap-3 border border-dashed px-6 py-10 transition-colors duration-300"
        :class="dragging ? 'border-acid/60 bg-acid/[0.04] text-acid' : 'border-line text-fg-faint hover:border-acid/50 hover:text-acid'"
        :disabled="busy"
        @click="choose"
    >
      <UIcon
          :name="reading ? 'i-lucide-loader' : 'i-lucide-file-archive'"
          class="size-5 transition-transform duration-500 ease-deck group-hover:-translate-y-0.5"
          :class="reading ? 'animate-spin' : ''"
      />
      <span class="font-mono text-[10px] uppercase tracking-[0.24em]">
        {{ reading ? 'Чтение архива' : pack ? 'Выбрать другой файл' : 'Выбрать файл модпака' }}
      </span>
      <span class="text-[12px] leading-relaxed text-fg-muted">
        Перетащите сюда или выберите <span class="text-fg">.mrpack</span> Modrinth, архив CurseForge
        или экспорт MultiMC / Prism
      </span>
    </button>

    <p v-if="pack" class="mt-3 truncate font-mono text-[10px] text-fg-faint" :title="pack.path">
      {{ pack.fileName }}
    </p>

    <div
        v-if="pack?.blocked"
        class="mt-6 flex items-start gap-2.5 border border-amber-400/30 bg-ink-900 px-4 py-3 text-[12px] leading-relaxed text-fg-muted"
    >
      <UIcon name="i-lucide-triangle-alert" class="mt-0.5 size-3.5 shrink-0 text-amber-400"/>
      Этот файл лаунчер установить не сможет: {{ pack.blocked }}.
    </div>

    <form v-if="pack" class="mt-6 space-y-8" @submit.prevent="run">
      <div class="space-y-5">
        <div>
          <label for="file-pack-name" class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            Название
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
            Описание
          </label>
          <UInput id="file-pack-description" v-model="description" placeholder="Необязательно" class="w-full"/>
        </div>
      </div>

      <dl class="grid grid-cols-2 border border-line sm:grid-cols-3">
        <div
            v-for="(fact, i) in facts"
            :key="fact.label"
            class="px-4 py-3"
            :class="i % 3 === 0 ? '' : 'sm:border-l sm:border-line'"
        >
          <dt class="font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">{{ fact.label }}</dt>
          <dd class="mt-1.5 truncate font-unbounded text-[13px] tracking-[-0.03em] text-fg" :title="fact.value">
            {{ fact.value }}
          </dd>
        </div>
      </dl>

      <p v-if="pack.kind === 'curseforge'" class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted">
        <UIcon name="i-lucide-hand" class="mt-0.5 size-3.5 shrink-0 text-amber-400"/>
        В архиве CurseForge лежат только ссылки на моды. Те, что авторы запретили скачивать лаунчерам,
        придётся скачать вручную - лаунчер попросит и покажет ссылки.
      </p>

      <p v-else-if="pack.kind === 'multimc'" class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted">
        <UIcon name="i-lucide-info" class="mt-0.5 size-3.5 shrink-0 text-fg-faint"/>
        Моды, конфиги и миры возьмутся прямо из архива, а клиент, библиотеки и Java лаунчер докачает сам.
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
        {{ importing ? 'Создание сборки' : 'Импортировать' }}
      </AppButton>
    </form>
  </div>
</template>
