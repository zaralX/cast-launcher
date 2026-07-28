<script setup lang="ts">
import type {Instance, InstanceSettings} from "~/types/instance";
import {emptyInstanceSettings, INSTANCE_TYPE_LABELS} from "~/types/instance";

definePageMeta({
  layout: "main"
})

type Tab = "general" | "java" | "logs"

const TABS: { key: Tab, label: string, icon: string, index: string }[] = [
  {key: "general", label: "Общее", icon: "i-lucide-box", index: "01"},
  {key: "java", label: "Java", icon: "i-lucide-cpu", index: "02"},
  {key: "logs", label: "Логи", icon: "i-lucide-scroll-text", index: "03"}
]

const route = useRoute()
const toast = useToast()

const instanceStore = useInstanceStore()

const instanceId = computed(() => String(route.params.id ?? ""))
const instance = computed(() => instanceStore.getInstance(instanceId.value))

const tab = ref<Tab>("general")
const saving = ref(false)

const running = computed(() => instanceStore.isRunning(instanceId.value))
const installing = computed(() => !!instanceStore.getInstall(instanceId.value))

const run = () => safeRun(
    () => instanceStore.runInstance(instanceId.value),
    {context: {instanceId: instanceId.value, action: "Запуск сборки"}}
)

const stop = () => safeRun(
    () => instanceStore.stopInstance(instanceId.value),
    {context: {instanceId: instanceId.value, action: "Остановка сборки"}}
)

const draft = ref({
  name: "",
  description: "",
  icon: "",
  settings: emptyInstanceSettings()
})

function snapshot(source: Instance) {
  return {
    name: source.name,
    description: source.description ?? "",
    icon: source.icon ?? "",
    settings: {...emptyInstanceSettings(), ...(source.settings ?? {})} as InstanceSettings
  }
}

watch(instanceId, () => {
  if (instance.value) draft.value = snapshot(instance.value)
}, {immediate: true})

watch(instance, (loaded, previous) => {
  if (loaded && !previous) draft.value = snapshot(loaded)
})

const dirty = computed(() => {
  if (!instance.value) return false
  return JSON.stringify(draft.value) !== JSON.stringify(snapshot(instance.value))
})

const canSave = computed(() => !!instance.value && dirty.value && !!draft.value.name.trim() && !saving.value)

async function save() {
  if (!canSave.value || !instance.value) return

  saving.value = true

  const result = await attempt(() => instanceStore.updateInstance(instanceId.value, {
    name: draft.value.name.trim(),
    description: draft.value.description.trim(),
    icon: draft.value.icon,
    settings: draft.value.settings
  }), {context: {instanceId: instanceId.value, action: "Сохранение настроек сборки"}})

  saving.value = false

  if (!result.ok) {
    toast.add({
      title: "Произошла ошибка",
      description: "Не получилось сохранить настройки сборки",
      color: "error",
      icon: "i-lucide-save"
    })
    return
  }

  draft.value = snapshot(result.value)

  toast.add({title: "Настройки сохранены", color: "success", icon: "i-lucide-save"})
}

function reset() {
  if (instance.value) draft.value = snapshot(instance.value)
}
</script>

<template>
  <div class="min-h-full w-full px-8 pb-16 pt-10 xl:px-14">
    <div v-if="instance" class="grid gap-10 lg:grid-cols-[15rem_minmax(0,1fr)] lg:gap-14">
      <aside class="lg:sticky lg:top-0 lg:self-start">
        <NuxtLink
            to="/main"
            class="group inline-flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint transition-colors duration-300 hover:text-acid"
        >
          <UIcon
              name="i-lucide-arrow-left"
              class="size-3.5 transition-transform duration-500 ease-deck group-hover:-translate-x-0.5"
          />
          Библиотека
        </NuxtLink>

        <div class="mt-4 flex items-start gap-4">
          <InstanceIcon :icon="draft.icon" :type="instance.type" size="md"/>

          <div class="min-w-0">
            <h1
                class="break-words font-unbounded text-[clamp(18px,2vw,24px)] font-bold leading-[1] tracking-[-0.055em] text-fg"
                :title="instance.name"
            >
              {{ instance.name }}<span class="text-acid">.</span>
            </h1>

            <p class="mt-2 font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
              {{ INSTANCE_TYPE_LABELS[instance.type] ?? instance.type }} · {{ instance.minecraftVersion }}
            </p>
          </div>
        </div>

        <nav class="mt-7 border-t border-line">
          <button
              v-for="item in TABS"
              :key="item.key"
              type="button"
              class="group relative flex w-full items-center gap-3 border-b border-line py-3 pl-4 pr-2 text-left transition-colors duration-300 hover:bg-ink-700"
              :class="tab === item.key ? 'text-fg' : 'text-fg-faint'"
              @click="tab = item.key"
          >
            <span
                class="absolute inset-y-0 left-0 w-[2px] bg-acid transition-transform duration-500 ease-deck"
                :class="tab === item.key ? 'scale-y-100' : 'scale-y-0 group-hover:scale-y-50 group-hover:bg-line-strong'"
            />

            <UIcon :name="item.icon" class="size-4 shrink-0" :class="tab === item.key ? 'text-acid' : ''"/>
            <span class="flex-1 font-mono text-[11px] uppercase tracking-[0.2em]">{{ item.label }}</span>
            <span class="font-mono text-[9px] tracking-[0.2em] text-fg-faint/60">{{ item.index }}</span>
          </button>
        </nav>

        <AppButton
            block
            class="mt-8 h-11 tracking-[0.2em]"
            icon="i-lucide-save"
            :loading="saving"
            :disabled="!canSave"
            @click="save"
        >
          {{ saving ? 'Сохранение' : 'Сохранить' }}
        </AppButton>

        <AppButton
            v-if="running"
            block
            class="mt-3 h-9 text-[10px] tracking-[0.18em]"
            icon="i-lucide-square"
            @click="stop"
        >
          Остановить игру
        </AppButton>

        <AppButton
            v-else-if="instance.installed"
            block
            class="mt-3 h-9 text-[10px] tracking-[0.18em]"
            icon="i-lucide-play"
            :disabled="installing"
            @click="run"
        >
          Играть
        </AppButton>

        <div v-if="dirty" class="mt-4 flex items-center justify-between gap-3">
          <span class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-amber-400">
            <span class="size-1.5 bg-amber-400 animate-blink"/>
            Не сохранено
          </span>

          <AppButton tone="quiet" class="text-[10px] tracking-[0.18em]" @click="reset">
            Сбросить
          </AppButton>
        </div>

        <p v-if="!draft.name.trim()" class="mt-4 font-mono text-[10px] uppercase tracking-[0.2em] text-amber-400">
          Название не может быть пустым
        </p>
      </aside>

      <div class="min-w-0">
        <InstanceGeneral
            v-show="tab === 'general'"
            :instance="instance"
            v-model:name="draft.name"
            v-model:description="draft.description"
            v-model:icon="draft.icon"
            class="animate-rise"
        />

        <InstanceJava v-show="tab === 'java'" v-model="draft.settings" class="animate-rise"/>

        <InstanceLogs v-if="tab === 'logs'" :instance-id="instance.id" class="animate-rise"/>
      </div>
    </div>

    <div v-else class="flex items-center gap-3 py-14">
      <span class="size-1.5 bg-fg-faint animate-blink"/>
      <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Сборка не найдена</p>
    </div>
  </div>
</template>
