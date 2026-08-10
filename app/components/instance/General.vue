<script setup lang="ts">
import type {Instance} from "~/types/instance";
import {formatLastPlayed, formatPlaytime, INSTANCE_TYPE_LABELS} from "~/types/instance";
import {call, type InstanceDir} from "~/types/backend";

const props = defineProps<{ instance: Instance }>()

const name = defineModel<string>("name", {required: true})
const description = defineModel<string>("description", {required: true})
const icon = defineModel<string>("icon", {required: true})

const pickerOpen = ref(false)

const instanceStore = useInstanceStore()
const router = useRouter()
const toast = useToast()

const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const {total, last} = usePlaytime(() => props.instance)

const facts = computed(() => [
  {label: "Идентификатор", value: props.instance.id},
  {label: "Загрузчик", value: INSTANCE_TYPE_LABELS[props.instance.type] ?? props.instance.type},
  {label: "Minecraft", value: props.instance.minecraftVersion},
  {label: "Версия загрузчика", value: props.instance.loaderVersion || "-"},
  {label: "Наиграно", value: formatPlaytime(total.value) || "-"},
  {
    label: "Последний запуск",
    value: formatLastPlayed(props.instance.playtime?.lastPlayedAt ?? 0) || "-"
  },
  {label: "Последняя сессия", value: formatPlaytime(last.value) || "-"}
])

const status = computed(() => {
  if (running.value) return {text: "Запущена", tone: "text-acid"}
  if (installing.value) return {text: "Устанавливается", tone: "text-fg-muted"}
  return props.instance.installed
      ? {text: "Установлена", tone: "text-fg-muted"}
      : {text: "Не установлена", tone: "text-amber-400"}
})

const DIRS: { target: InstanceDir, label: string, icon: string }[] = [
  {target: "root", label: "Папка сборки", icon: "i-lucide-folder"},
  {target: "minecraft", label: "minecraft", icon: "i-lucide-folder-open"},
  {target: "logs", label: "Логи", icon: "i-lucide-folder-clock"}
]

const openDir = (target: InstanceDir) => safeRun(
    () => call("open_instance_dir", {instanceId: props.instance.id, target}),
    {context: {instanceId: props.instance.id, action: "Открытие папки сборки"}}
)

const reinstall = () => safeRun(
    () => instanceStore.installInstance(props.instance.id),
    {context: {instanceId: props.instance.id, action: "Переустановка сборки"}}
)

const removeOpen = ref(false)
const removing = ref(false)

async function remove() {
  if (removing.value) return
  removing.value = true

  const result = await attempt(
      () => instanceStore.deleteInstance(props.instance.id),
      {context: {instanceId: props.instance.id, action: "Удаление сборки"}}
  )

  removing.value = false

  if (!result.ok) return

  removeOpen.value = false
  toast.add({title: "Сборка удалена", color: "success", icon: "i-lucide-trash-2"})
  await router.push("/main")
}
</script>

<template>
  <div class="space-y-6">
    <SettingsPanel
        index="01"
        title="Общее"
        icon="i-lucide-box"
    >
      <div class="space-y-7">
        <div class="flex items-center gap-5">
          <InstanceIcon :icon="icon" :type="instance.type" size="lg"/>

          <div class="min-w-0 flex-1">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Иконка</p>
            <p class="mt-2 truncate font-mono text-[11px] text-fg-muted" :title="icon">
              {{ icon || "Метка по типу загрузчика" }}
            </p>

            <div class="mt-3 flex items-center gap-3">
              <AppButton
                  class="h-8 px-3.5 text-[10px] tracking-[0.18em]"
                  icon="i-lucide-image"
                  @click="pickerOpen = true"
              >
                Выбрать иконку
              </AppButton>

              <AppButton
                  v-if="icon"
                  tone="quiet"
                  class="text-[10px] tracking-[0.18em]"
                  icon="i-lucide-x"
                  @click="icon = ''"
              >
                Убрать
              </AppButton>
            </div>
          </div>
        </div>

        <SettingsField label="Название">
          <UInput
              v-model="name"
              placeholder="Например, Hardcore Survival"
              class="w-full"
          />
        </SettingsField>

<!--        <SettingsField label="Описание">-->
<!--          <UInput v-model="description" placeholder="Необязательно" class="w-full"/>-->
<!--        </SettingsField>-->

        <dl class="grid gap-x-6 gap-y-4 border-t border-line pt-6 sm:grid-cols-2">
          <div v-for="fact in facts" :key="fact.label" class="min-w-0">
            <dt class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ fact.label }}</dt>
            <dd class="mt-1.5 truncate font-mono text-[12px] text-fg-muted" :title="fact.value">{{ fact.value }}</dd>
          </div>

          <div class="min-w-0">
            <dt class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Состояние</dt>
            <dd class="mt-1.5 font-mono text-[12px]" :class="status.tone">{{ status.text }}</dd>
          </div>
        </dl>
      </div>
    </SettingsPanel>

    <SettingsPanel
        index="02"
        title="Файлы и обслуживание"
        icon="i-lucide-hard-drive"
    >
      <div class="space-y-7">
        <div class="flex flex-wrap gap-3">
          <AppButton
              v-for="dir in DIRS"
              :key="dir.target"
              class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
              :icon="dir.icon"
              @click="openDir(dir.target)"
          >
            {{ dir.label }}
          </AppButton>
        </div>

        <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
              {{ instance.installed ? 'Переустановка' : 'Установка' }}
            </p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              Скачивает клиент, библиотеки и ресурсы заново. Миры и конфиги в minecraft остаются на месте.
              <template v-if="instance.pack">
                Файлы модпака тоже проверяются - сменить его версию можно на вкладке "Модпак".
              </template>
              <template v-else-if="instance.localPack">
                Файлы модпака разложатся заново из архива, с которым сборку импортировали.
              </template>
            </p>
          </div>

          <AppButton
              class="h-9 shrink-0 px-3.5 text-[10px] tracking-[0.18em]"
              :icon="instance.installed ? 'i-lucide-refresh-cw' : 'i-lucide-arrow-down-to-line'"
              :disabled="installing || running"
              @click="reinstall"
          >
            {{ installing ? 'Идёт установка' : instance.installed ? 'Переустановить' : 'Установить' }}
          </AppButton>
        </div>

        <div class="flex items-center justify-between gap-6 border-t border-red-400/20 pt-6">
          <div class="min-w-0">
            <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-red-400/80">Удаление</p>
            <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">
              Сборка и все её файлы, включая миры, будут удалены с диска безвозвратно.
            </p>
          </div>

          <UButton
              color="neutral"
              variant="ghost"
              class="h-9 shrink-0 justify-center border border-red-400/30 px-3.5 text-[10px] tracking-[0.18em] text-red-400 transition-colors duration-300 hover:bg-red-500 hover:text-white"
              icon="i-lucide-trash-2"
              :disabled="running"
              @click="removeOpen = true"
          >
            Удалить
          </UButton>
        </div>

        <p v-if="running" class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
          Сборка запущена - сначала закройте игру
        </p>
      </div>
    </SettingsPanel>

    <UModal v-model:open="pickerOpen" title="Иконка сборки" :ui="{ content: 'max-w-3xl' }">
      <template #body>
        <InstanceIconPicker v-model="icon"/>
      </template>
    </UModal>

    <UModal v-model:open="removeOpen" title="Удаление сборки">
      <template #body>
        <div class="space-y-6">
          <p class="text-[13px] leading-relaxed text-fg-muted">
            Удалить сборку «<span class="text-fg">{{ instance.name }}</span>» вместе со всеми её файлами? Действие
            необратимо.
          </p>

          <div class="flex justify-end gap-3">
            <AppButton
                tone="quiet"
                class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
                :disabled="removing"
                @click="removeOpen = false"
            >
              Отмена
            </AppButton>

            <UButton
                color="neutral"
                variant="ghost"
                class="h-9 justify-center border border-red-400/30 px-3.5 text-[10px] tracking-[0.18em] text-red-400 transition-colors duration-300 hover:bg-red-500 hover:text-white"
                icon="i-lucide-trash-2"
                :loading="removing"
                @click="remove"
            >
              {{ removing ? 'Удаление' : 'Удалить навсегда' }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
