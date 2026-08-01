<script setup lang="ts">
import type {ContextMenuItem} from "@nuxt/ui";
import type {Instance} from "~/types/instance";
import type {InstanceDir} from "~/types/backend";

const props = defineProps<{
  instance: Instance
  selected?: boolean
}>()

const emit = defineEmits<{
  select: [id: string]
  remove: [id: string]
}>()

const actions = useInstanceActions()

const state = computed(() => actions.stateOf(props.instance))
const install = computed(() => actions.installOf(props.instance.id))

const DIRS: InstanceDir[] = ["root", "minecraft", "logs"]

const items = computed<ContextMenuItem[][]>(() => {
  const id = props.instance.id

  const primary: ContextMenuItem[] = []

  if (state.value === "running") {
    primary.push({label: "Остановить", icon: "i-lucide-square", onSelect: () => actions.stop(id)})
  } else if (state.value === "installing") {
    primary.push({label: "Отменить установку", icon: "i-lucide-x", onSelect: () => actions.cancelInstall(id)})
  } else if (state.value === "ready") {
    primary.push({label: "Играть", icon: "i-lucide-play", onSelect: () => actions.play(id)})
  } else {
    primary.push({label: "Загрузить", icon: "i-lucide-arrow-down-to-line", onSelect: () => actions.install(id)})
  }

  return [
    primary,
    [
      {label: "Настройки", icon: "i-lucide-settings", onSelect: () => navigateTo(`/instance/${id}`)},
      ...DIRS.map(target => ({
        label: INSTANCE_DIR_LABELS[target],
        icon: "i-lucide-folder-open",
        onSelect: () => actions.openDir(id, target)
      }))
    ],
    [
      {label: "Удалить", icon: "i-lucide-trash-2", color: "error" as const, onSelect: () => emit("remove", id)}
    ]
  ]
})

function select() {
  emit("select", props.instance.id)
}
</script>

<template>
  <UContextMenu :items="items" size="sm">
    <button
        type="button"
        :aria-pressed="!!selected"
        :title="instance.name"
        class="group relative flex w-full cursor-pointer flex-col items-center gap-2 border p-2.5 text-center transition-colors duration-300"
        :class="selected
          ? 'border-acid bg-ink-700'
          : 'border-transparent hover:border-line-strong hover:bg-ink-800'"
        @click="select"
        @dblclick="actions.primary(instance)"
        @contextmenu="select"
    >
      <span class="relative">
        <InstanceIcon
            :icon="instance.icon"
            :type="instance.type"
            size="md"
            :bordered="false"
            class="text-fg-faint transition-colors duration-300"
            :class="selected ? 'text-acid' : 'group-hover:text-fg-muted'"
        />

        <span
            v-if="state === 'running'"
            class="absolute -right-1 -top-1 grid size-2 place-items-center"
            title="Запущено"
        >
          <span class="absolute size-2 bg-acid animate-breathe"/>
          <span class="size-2 bg-acid"/>
        </span>

        <span
            v-else-if="state === 'absent'"
            class="absolute -right-1 -top-1 size-2 bg-fg-faint"
            title="Не загружена"
        />
      </span>

      <span
          class="line-clamp-2 w-full break-words text-[11px] leading-tight transition-colors duration-300"
          :class="selected ? 'text-fg' : 'text-fg-muted group-hover:text-fg'"
      >
        {{ instance.name }}
      </span>

      <span v-if="state === 'installing'" class="relative block h-px w-full overflow-hidden bg-line">
        <span
            v-if="install?.progress == null"
            class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"
        />
        <span
            v-else
            class="absolute inset-y-0 left-0 bg-acid transition-[width] duration-500 ease-deck"
            :style="{ width: `${Math.round(install.progress * 100)}%` }"
        />
      </span>
    </button>
  </UContextMenu>
</template>
