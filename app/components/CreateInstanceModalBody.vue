<script setup lang="ts">
import type {InstanceType} from "~/types/instance";
import {v4} from "uuid";
import {call} from "~/types/backend";
import {LauncherError} from "~/types/error";

const emit = defineEmits<{ created: [] }>()

const instancesStore = useInstanceStore()

const loading = ref(true)
const loadError = ref<LauncherError | null>(null)
const creating = ref(false)

const minecraftVersions = ref<string[]>([])
const fabricLoaderVersions = ref<string[]>([])
const forgeVersions = ref<string[]>([])

const name = ref("")
const description = ref("")
const instanceType = ref<InstanceType>("vanilla")
const minecraftVersion = ref<string>("")
const fabricLoader = ref<string>("latest")
const forgeLoader = ref<string>("latest")

const TYPES: { value: InstanceType; label: string; mark: string }[] = [
  {value: "vanilla", label: "Vanilla", mark: "VA"},
  {value: "fabric", label: "Fabric", mark: "FA"},
  {value: "forge", label: "Forge", mark: "FO"}
]

const filteredForgeVersions = computed(() =>
    forgeVersions.value.filter((v: string) => v.startsWith(minecraftVersion.value))
)

const canCreate = computed(() =>
    !loading.value && !loadError.value && !creating.value && name.value.trim().length > 0
)

async function loadMetadata() {
  loading.value = true
  loadError.value = null

  try {
    const [manifest, fabric, forge] = await Promise.all([
      call("list_minecraft_versions"),
      call("list_fabric_versions"),
      call("list_forge_versions")
    ])

    minecraftVersions.value = manifest.versions
        .filter(version => version.type === "release")
        .map(version => version.id)
    minecraftVersion.value = manifest.latest.release ?? minecraftVersions.value[0] ?? ""

    fabricLoaderVersions.value = fabric
    fabricLoader.value = fabric[0] ?? "latest"

    forgeVersions.value = forge
    forgeLoader.value = forge[0] ?? "latest"
  } catch (e) {
    loadError.value = captureError(e, {code: "NETWORK", context: {action: "Загрузка списка версий"}})
  } finally {
    loading.value = false
  }
}

onMounted(loadMetadata)

function loaderVersion(): string | undefined {
  if (instanceType.value === "fabric") return fabricLoader.value
  if (instanceType.value === "forge") return forgeLoader.value
  return undefined
}

const createInstance = async () => {
  if (!canCreate.value) return

  creating.value = true

  const result = await attempt(() => instancesStore.createInstance({
    id: v4(),
    name: name.value.trim(),
    description: description.value.trim(),
    type: instanceType.value,
    minecraftVersion: minecraftVersion.value,
    loaderVersion: loaderVersion(),
    version: 1
  }))

  creating.value = false

  if (result.ok) emit("created")
}
</script>

<template>
  <div>
    <div v-if="loading" class="flex flex-col items-center gap-4 py-14">
      <span class="relative block h-px w-40 overflow-hidden bg-line">
        <span class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"/>
      </span>
      <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Загрузка списка версий</p>
    </div>

    <div v-else-if="loadError" class="border border-red-400/30 bg-ink-900 p-5">
      <div class="flex items-start gap-3">
        <UIcon name="i-lucide-wifi-off" class="mt-0.5 size-4 shrink-0 text-red-400"/>
        <div class="min-w-0">
          <p class="text-[13px] font-medium text-fg">{{ loadError.title }}</p>
          <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">{{ loadError.hint ?? loadError.message }}</p>
          <button
              type="button"
              class="mt-4 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted transition-colors duration-300 hover:text-acid"
              @click="loadMetadata"
          >
            <UIcon name="i-lucide-rotate-cw" class="size-3"/>
            Повторить
          </button>
        </div>
      </div>
    </div>

    <form v-else class="space-y-8" @submit.prevent="createInstance">
      <div class="space-y-5">
        <div>
          <label
              for="instance-name"
              class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            Название
          </label>
          <UInput
              id="instance-name"
              v-model="name"
              placeholder="Например, Hardcore Survival"
              size="lg"
              class="w-full"
              :ui="{ base: 'font-unbounded text-[15px] tracking-[-0.03em]' }"
          />
        </div>

        <div>
          <label
              for="instance-description"
              class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            Описание
          </label>
          <UInput id="instance-description" v-model="description" placeholder="Необязательно" class="w-full"/>
        </div>
      </div>

      <div>
        <span class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Загрузчик</span>
        <div class="grid grid-cols-3 border border-line">
          <button
              v-for="(type, i) in TYPES"
              :key="type.value"
              type="button"
              class="group relative flex flex-col items-center gap-1.5 py-4 transition-colors duration-300"
              :class="[
                i > 0 ? 'border-l border-line' : '',
                instanceType === type.value ? 'bg-ink-700 text-fg' : 'text-fg-faint hover:bg-ink-700/50 hover:text-fg-muted'
              ]"
              @click="instanceType = type.value"
          >
            <span
                class="absolute inset-x-0 top-0 h-px origin-center scale-x-0 bg-acid transition-transform duration-500 ease-deck"
                :class="instanceType === type.value ? 'scale-x-100' : ''"
            />
            <span
                class="font-mono text-[10px] tracking-[0.1em] transition-colors duration-300"
                :class="instanceType === type.value ? 'text-acid' : ''"
            >
              {{ type.mark }}
            </span>
            <span class="font-unbounded text-[12px] tracking-[-0.02em]">{{ type.label }}</span>
          </button>
        </div>
      </div>

      <div class="grid gap-5" :class="instanceType === 'vanilla' ? 'grid-cols-1' : 'grid-cols-2'">
        <div>
          <label class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Minecraft</label>
          <USelect v-model="minecraftVersion" :items="minecraftVersions" class="w-full"/>
        </div>

        <div v-if="instanceType === 'fabric'">
          <label class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Fabric Loader</label>
          <USelect v-model="fabricLoader" :items="fabricLoaderVersions" class="w-full"/>
        </div>

        <div v-if="instanceType === 'forge'">
          <label class="mb-2 block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Forge</label>
          <USelect v-model="forgeLoader" :items="filteredForgeVersions" class="w-full"/>
        </div>
      </div>

      <button
          type="submit"
          :disabled="!canCreate"
          class="group/act relative flex h-11 w-full items-center justify-center overflow-hidden border border-line font-mono text-[11px] uppercase tracking-[0.2em] text-fg transition-colors duration-300 hover:border-acid hover:text-on-acid disabled:pointer-events-none disabled:opacity-30"
      >
        <span
            class="absolute inset-0 origin-left scale-x-0 bg-acid transition-transform duration-500 ease-deck group-hover/act:scale-x-100"
            aria-hidden="true"
        />
        <span class="relative flex items-center gap-2">
          <UIcon
              :name="creating ? 'i-lucide-loader-circle' : 'i-lucide-plus'"
              class="size-3.5"
              :class="creating ? 'animate-spin' : 'transition-transform duration-500 group-hover/act:rotate-90'"
          />
          {{ creating ? 'Создание' : 'Создать сборку' }}
        </span>
      </button>
    </form>
  </div>
</template>
