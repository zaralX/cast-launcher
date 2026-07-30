<script setup lang="ts">
import type {CastPackFile, CastPackManifest, CastPackMod} from "~/types/castpack"
import {emptyFile, emptyManifest, emptyMod, manifestJson} from "~/types/castpack"
import {INSTANCE_TYPE_LABELS, PACK_PROVIDER_LABELS, type InstanceType, type PackProvider} from "~/types/instance"
import {call} from "~/types/backend"

definePageMeta({
  layout: "main"
})

const toast = useToast()

const manifest = ref<CastPackManifest>(emptyManifest())

const useLoader = ref(false)
const useBase = ref(false)
const deleteText = ref("")

const checking = ref(false)
const saving = ref(false)
const probing = ref<number | null>(null)
const problem = ref<string | null>(null)
const checked = ref(false)

const importOpen = ref(false)
const importText = ref("")

const LOADERS = (Object.keys(INSTANCE_TYPE_LABELS) as InstanceType[])
    .map(value => ({label: INSTANCE_TYPE_LABELS[value], value}))

const PROVIDERS = (Object.keys(PACK_PROVIDER_LABELS) as PackProvider[])
    .map(value => ({label: PACK_PROVIDER_LABELS[value], value}))

const MODES = [
  {label: "always - файл сборки, перезаписывается", value: "always"},
  {label: "once - положить один раз и не трогать", value: "once"}
]

watch(useLoader, on => {
  manifest.value.loader = on ? {type: "fabric", version: ""} : undefined
})

watch(useBase, on => {
  manifest.value.base = on ? {provider: "modrinth", projectId: "", versionId: ""} : undefined
})

watch(deleteText, text => {
  manifest.value.delete = text.split("\n").map(line => line.trim()).filter(Boolean)
})

const json = computed(() => manifestJson(manifest.value))

watch(json, () => {
  checked.value = false
  problem.value = null
})

const addMod = () => manifest.value.mods.push(emptyMod())
const addFile = () => manifest.value.files.push(emptyFile())

const removeMod = (index: number) => manifest.value.mods.splice(index, 1)
const removeFile = (index: number) => manifest.value.files.splice(index, 1)

const isDirect = (mod: CastPackMod) => !!mod.url.trim()

function toggleModKind(mod: CastPackMod, direct: boolean) {
  if (direct) {
    mod.provider = undefined
    mod.projectId = ""
    mod.versionId = ""
    mod.url = mod.url || "https://"
  } else {
    mod.provider = "modrinth"
    mod.url = ""
    mod.path = ""
    mod.sha1 = undefined
    mod.size = undefined
  }
}

async function probeMod(index: number) {
  const mod = manifest.value.mods[index]
  if (!mod) return

  probing.value = index

  try {
    if (isDirect(mod)) {
      const probed = await call("castpack_probe_file", {url: mod.url.trim()})

      mod.sha1 = probed.sha1
      mod.size = probed.size
      if (!mod.path.trim()) mod.path = `mods/${probed.fileName}`

      toast.add({title: `Файл проверен: ${probed.fileName}`, color: "success", icon: "i-lucide-check"})
      return
    }

    if (!mod.provider) return

    const probed = await call("castpack_probe_mod", {
      provider: mod.provider,
      projectId: mod.projectId.trim(),
      versionId: mod.versionId.trim()
    })

    toast.add({
      title: probed.blocked ? "Файл придётся качать вручную" : `Мод ляжет в ${probed.path}`,
      description: probed.blocked ? "Автор запретил скачивание через сторонние лаунчеры" : undefined,
      color: probed.blocked ? "warning" : "success",
      icon: probed.blocked ? "i-lucide-hand" : "i-lucide-check"
    })
  } catch (e) {
    captureError(e, {code: "NETWORK", context: {action: "Проверка мода сборки"}})
  } finally {
    probing.value = null
  }
}

async function probeFile(index: number) {
  const file = manifest.value.files[index]
  if (!file?.url.trim()) return

  probing.value = 1000 + index

  try {
    const probed = await call("castpack_probe_file", {url: file.url.trim()})

    file.sha1 = probed.sha1
    file.size = probed.size
    if (!file.path.trim()) file.path = probed.fileName

    toast.add({title: `Файл проверен: ${probed.fileName}`, color: "success", icon: "i-lucide-check"})
  } catch (e) {
    captureError(e, {code: "NETWORK", context: {action: "Проверка файла сборки"}})
  } finally {
    probing.value = null
  }
}

async function validate() {
  checking.value = true
  problem.value = null

  const result = await attempt(
      () => call("castpack_validate", {json: json.value}),
      {toast: false, context: {action: "Проверка манифеста CastPack"}}
  )

  checking.value = false
  checked.value = result.ok

  if (result.ok) {
    toast.add({title: "Манифест корректен", color: "success", icon: "i-lucide-check"})
  } else {
    problem.value = result.error.message
  }
}

async function copyJson() {
  if (await copyToClipboard(json.value)) {
    toast.add({title: "JSON скопирован", color: "success", icon: "i-lucide-copy"})
    return
  }

  toast.add({
    title: "Скопировать не вышло",
    description: "Сохраните манифест в файл - так надёжнее",
    color: "error",
    icon: "i-lucide-copy"
  })
}

async function saveJson() {
  if (saving.value) return

  saving.value = true
  problem.value = null

  const result = await attempt(
      () => call("castpack_save_manifest", {json: json.value}),
      {toast: false, context: {action: "Сохранение манифеста CastPack"}}
  )

  saving.value = false

  if (!result.ok) {
    checked.value = false
    problem.value = result.error.message
    return
  }

  if (!result.value) return

  checked.value = true

  toast.add({
    title: "Манифест сохранён",
    description: result.value,
    color: "success",
    icon: "i-lucide-save"
  })
}

function load() {
  const result = tryParse(importText.value)

  if (!result) {
    toast.add({title: "Это не похоже на манифест", color: "error", icon: "i-lucide-file-warning"})
    return
  }

  manifest.value = result
  useLoader.value = !!result.loader
  useBase.value = !!result.base
  deleteText.value = result.delete.join("\n")

  importOpen.value = false
  importText.value = ""

  toast.add({title: "Манифест загружен", color: "success", icon: "i-lucide-file-input"})
}

function tryParse(text: string): CastPackManifest | null {
  try {
    const raw = JSON.parse(text) as Partial<CastPackManifest>
    if (!raw || typeof raw !== "object") return null

    return {
      ...emptyManifest(),
      ...raw,
      mods: (raw.mods ?? []).map(mod => ({...emptyMod(), ...mod})),
      files: (raw.files ?? []).map(file => ({...emptyFile(), ...file})),
      delete: raw.delete ?? [],
      settings: raw.settings ?? {}
    }
  } catch {
    return null
  }
}

function reset() {
  manifest.value = emptyManifest()
  useLoader.value = false
  useBase.value = false
  deleteText.value = ""
  problem.value = null
  checked.value = false
}
</script>

<template>
  <div class="min-h-full w-full px-8 pb-16 xl:px-14">
    <div class="grid gap-10 lg:grid-cols-[15rem_minmax(0,1fr)] lg:gap-14">
      <aside class="lg:sticky pt-10 lg:top-0 lg:self-start">
        <NuxtLink
            to="/settings"
            class="group inline-flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint transition-colors duration-300 hover:text-acid"
        >
          <UIcon
              name="i-lucide-arrow-left"
              class="size-3.5 transition-transform duration-500 ease-deck group-hover:-translate-x-0.5"
          />
          Настройки
        </NuxtLink>

        <h1 class="mt-4 font-unbounded text-[clamp(22px,2.6vw,30px)] font-bold leading-[0.95] tracking-[-0.055em] text-fg">
          Редактор<span class="text-acid">.</span>
        </h1>

        <p class="mt-5 text-[12px] leading-relaxed text-fg-muted">
          Собирает manifest.json для сборки CastPack. Проверка гоняет его через тот же
          разбор, что и установка.
        </p>

        <AppButton
            block
            class="mt-8 h-11 tracking-[0.2em]"
            icon="i-lucide-shield-check"
            :loading="checking"
            @click="validate"
        >
          {{ checking ? 'Проверка' : 'Проверить' }}
        </AppButton>

        <AppButton
            block
            class="mt-3 h-9 text-[10px] tracking-[0.18em]"
            icon="i-lucide-save"
            :loading="saving"
            @click="saveJson"
        >
          Сохранить в файл
        </AppButton>

        <AppButton
            block
            tone="quiet"
            class="mt-2 h-9 text-[10px] tracking-[0.18em]"
            icon="i-lucide-copy"
            @click="copyJson"
        >
          Скопировать JSON
        </AppButton>

        <AppButton
            block
            tone="quiet"
            class="mt-2 h-9 text-[10px] tracking-[0.18em]"
            icon="i-lucide-file-input"
            @click="importOpen = true"
        >
          Загрузить JSON
        </AppButton>

        <AppButton
            block
            tone="quiet"
            class="mt-2 h-9 text-[10px] tracking-[0.18em]"
            icon="i-lucide-eraser"
            @click="reset"
        >
          Очистить
        </AppButton>

        <p
            v-if="checked"
            class="mt-5 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.2em] text-acid"
        >
          <span class="size-1.5 bg-acid"/>
          Манифест корректен
        </p>

        <p v-if="problem" class="mt-5 text-[12px] leading-relaxed text-amber-400">{{ problem }}</p>
      </aside>

      <div class="min-w-0 space-y-6 pt-10">
        <SettingsPanel index="01" title="Сборка" icon="i-lucide-badge-info" class="animate-rise">
          <div class="grid gap-6 sm:grid-cols-2">
            <SettingsField label="Идентификатор" hint="Латиница, цифры, дефис. По нему сборка живёт в каталоге">
              <UInput v-model="manifest.id" placeholder="zaralx-rpg" class="w-full"/>
            </SettingsField>

            <SettingsField label="Название">
              <UInput v-model="manifest.name" placeholder="zaralX RPG" class="w-full"/>
            </SettingsField>

            <SettingsField label="Версия" hint="Меняется - лаунчер обновит сборку у всех">
              <UInput v-model="manifest.version" placeholder="1.4.2" class="w-full"/>
            </SettingsField>

            <SettingsField label="Версия Minecraft" hint="Обязательна, если нет базового модпака">
              <UInput v-model="manifest.minecraft" placeholder="1.20.1" class="w-full"/>
            </SettingsField>
          </div>

          <SettingsField class="mt-6" label="Что изменилось" hint="Покажется игроку после обновления">
            <UTextarea v-model="manifest.changelog" :rows="3" class="w-full"/>
          </SettingsField>

          <SettingsField class="mt-6" label="Рекомендуемая память, МБ" hint="Ставится один раз при первой установке">
            <UInput v-model.number="manifest.settings.recommendedRam" type="number" placeholder="6144" class="w-full"/>
          </SettingsField>
        </SettingsPanel>

        <SettingsPanel index="02" title="Основа" icon="i-lucide-package" class="animate-rise">
          <SettingsField label="Базовый модпак" hint="Моды сборки лягут поверх него">
            <USwitch v-model="useBase"/>
          </SettingsField>

          <div v-if="manifest.base" class="mt-6 grid gap-6 sm:grid-cols-3">
            <SettingsField label="Каталог">
              <USelect v-model="manifest.base.provider" :items="PROVIDERS" value-key="value" class="w-full"/>
            </SettingsField>

            <SettingsField label="Проект">
              <UInput v-model="manifest.base.projectId" placeholder="925200" class="w-full"/>
            </SettingsField>

            <SettingsField label="Версия">
              <UInput v-model="manifest.base.versionId" placeholder="5432100" class="w-full"/>
            </SettingsField>
          </div>

          <div class="mt-8 border-t border-line pt-7">
            <SettingsField label="Задать загрузчик" hint="Без базового пака - обязательно, с ним - переопределяет пак">
              <USwitch v-model="useLoader"/>
            </SettingsField>

            <div v-if="manifest.loader" class="mt-6 grid gap-6 sm:grid-cols-2">
              <SettingsField label="Загрузчик">
                <USelect v-model="manifest.loader.type" :items="LOADERS" value-key="value" class="w-full"/>
              </SettingsField>

              <SettingsField label="Версия загрузчика">
                <UInput v-model="manifest.loader.version" placeholder="0.16.0" class="w-full"/>
              </SettingsField>
            </div>
          </div>
        </SettingsPanel>

        <SettingsPanel index="03" title="Моды" icon="i-lucide-blocks" class="animate-rise">
          <p class="text-[12px] leading-relaxed text-fg-muted">
            Мод берётся либо из каталога по паре «проект + версия», либо по прямой ссылке.
            Версии пинятся жёстко: пока они не изменились в манифесте, у игроков ничего не поедет.
          </p>

          <ul v-if="manifest.mods.length" class="mt-6 space-y-4">
            <li v-for="(mod, index) in manifest.mods" :key="index" class="border border-line p-4">
              <div class="flex items-center justify-between gap-4">
                <div class="flex items-center gap-4">
                  <span class="font-mono text-[10px] tracking-[0.2em] text-acid">
                    {{ String(index + 1).padStart(2, "0") }}
                  </span>

                  <UCheckbox
                      :model-value="isDirect(mod)"
                      label="Прямая ссылка"
                      @update:model-value="value => toggleModKind(mod, !!value)"
                  />

                  <UCheckbox v-model="mod.optional" label="Опциональный"/>
                </div>

                <div class="flex items-center gap-2">
                  <AppButton
                      tone="quiet"
                      class="text-[10px] tracking-[0.16em]"
                      icon="i-lucide-search-check"
                      :loading="probing === index"
                      @click="probeMod(index)"
                  >
                    Проверить
                  </AppButton>

                  <AppButton
                      tone="quiet"
                      class="text-[10px] tracking-[0.16em]"
                      icon="i-lucide-trash-2"
                      @click="removeMod(index)"
                  >
                    Убрать
                  </AppButton>
                </div>
              </div>

              <div v-if="isDirect(mod)" class="mt-4 grid gap-4 sm:grid-cols-2">
                <SettingsField label="Ссылка">
                  <UInput v-model="mod.url" placeholder="https://cdn.zaralx.ru/mods/core-1.2.jar" class="w-full"/>
                </SettingsField>

                <SettingsField label="Путь в игре">
                  <UInput v-model="mod.path" placeholder="mods/core-1.2.jar" class="w-full"/>
                </SettingsField>

                <SettingsField label="sha1" hint="Обязателен: по нему видно, что мод обновился">
                  <UInput v-model="mod.sha1" class="w-full"/>
                </SettingsField>

                <SettingsField label="Размер, байт">
                  <UInput v-model.number="mod.size" type="number" class="w-full"/>
                </SettingsField>
              </div>

              <div v-else class="mt-4 grid gap-4 sm:grid-cols-3">
                <SettingsField label="Каталог">
                  <USelect v-model="mod.provider" :items="PROVIDERS" value-key="value" class="w-full"/>
                </SettingsField>

                <SettingsField label="Проект">
                  <UInput v-model="mod.projectId" placeholder="AANobbMI" class="w-full"/>
                </SettingsField>

                <SettingsField
                    label="Версия"
                    :hint="mod.provider === 'curseforge' ? 'id файла' : 'id версии'"
                >
                  <UInput v-model="mod.versionId" class="w-full"/>
                </SettingsField>
              </div>
            </li>
          </ul>

          <AppButton
              tone="quiet"
              class="mt-6 text-[10px] tracking-[0.16em]"
              icon="i-lucide-plus"
              @click="addMod"
          >
            Добавить мод
          </AppButton>
        </SettingsPanel>

        <SettingsPanel index="04" title="Файлы" icon="i-lucide-file-cog" class="animate-rise">
          <p class="text-[12px] leading-relaxed text-fg-muted">
            Конфиги, options.txt, список серверов. Режим <span class="text-fg">always</span> - файл
            принадлежит сборке и перезаписывается, <span class="text-fg">once</span> - кладётся один
            раз и дальше остаётся за игроком.
          </p>

          <ul v-if="manifest.files.length" class="mt-6 space-y-4">
            <li v-for="(file, index) in manifest.files" :key="index" class="border border-line p-4">
              <div class="flex items-center justify-between gap-4">
                <span class="font-mono text-[10px] tracking-[0.2em] text-acid">
                  {{ String(index + 1).padStart(2, "0") }}
                </span>

                <div class="flex items-center gap-2">
                  <AppButton
                      tone="quiet"
                      class="text-[10px] tracking-[0.16em]"
                      icon="i-lucide-search-check"
                      :loading="probing === 1000 + index"
                      @click="probeFile(index)"
                  >
                    Проверить
                  </AppButton>

                  <AppButton
                      tone="quiet"
                      class="text-[10px] tracking-[0.16em]"
                      icon="i-lucide-trash-2"
                      @click="removeFile(index)"
                  >
                    Убрать
                  </AppButton>
                </div>
              </div>

              <div class="mt-4 grid gap-4 sm:grid-cols-2">
                <SettingsField label="Путь в игре">
                  <UInput v-model="file.path" placeholder="config/rpg.toml" class="w-full"/>
                </SettingsField>

                <SettingsField label="Ссылка">
                  <UInput v-model="file.url" placeholder="https://cdn.zaralx.ru/files/rpg.toml" class="w-full"/>
                </SettingsField>

                <SettingsField label="sha1">
                  <UInput v-model="file.sha1" class="w-full"/>
                </SettingsField>

                <SettingsField label="Режим">
                  <USelect v-model="file.mode" :items="MODES" value-key="value" class="w-full"/>
                </SettingsField>
              </div>
            </li>
          </ul>

          <AppButton
              tone="quiet"
              class="mt-6 text-[10px] tracking-[0.16em]"
              icon="i-lucide-plus"
              @click="addFile"
          >
            Добавить файл
          </AppButton>
        </SettingsPanel>

        <SettingsPanel index="05" title="Удалить из игры" icon="i-lucide-trash-2" class="animate-rise">
          <SettingsField
              label="Пути, по одному на строку"
              hint="Так убирают мусор базового модпака и моды, которые сборка заменила своими"
          >
            <UTextarea v-model="deleteText" :rows="4" placeholder="mods/optifine.jar" class="w-full"/>
          </SettingsField>
        </SettingsPanel>

        <SettingsPanel index="06" title="manifest.json" icon="i-lucide-braces" class="animate-rise">
          <pre class="max-h-[28rem] overflow-auto border border-line bg-ink-900 p-4 font-mono text-[11px] leading-relaxed text-fg-muted">{{ json }}</pre>
        </SettingsPanel>
      </div>
    </div>

    <UModal v-model:open="importOpen" title="Загрузить манифест">
      <template #body>
        <div class="space-y-5">
          <p class="text-[12px] leading-relaxed text-fg-muted">
            Вставьте содержимое manifest.json, чтобы продолжить редактирование.
          </p>

          <UTextarea v-model="importText" :rows="12" class="w-full font-mono text-[11px]"/>

          <div class="flex justify-end gap-2">
            <AppButton tone="quiet" class="text-[10px] tracking-[0.18em]" @click="importOpen = false">
              Отмена
            </AppButton>

            <AppButton class="text-[10px] tracking-[0.18em]" icon="i-lucide-file-input" @click="load">
              Загрузить
            </AppButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
