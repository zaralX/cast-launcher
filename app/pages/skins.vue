<script setup lang="ts">
import {storeToRefs} from "pinia"
import {getCurrentWebview} from "@tauri-apps/api/webview"
import type {UnlistenFn} from "@tauri-apps/api/event"
import type {SkinEntry, SkinPose, SkinVariant} from "~/types/skin"
import {SOURCE_LABELS, VARIANT_HINTS, VARIANT_LABELS} from "~/types/skin"

definePageMeta({
  layout: "main"
})

const toast = useToast()

const skinStore = useSkinStore()
const {library, capes, draft, loading, saving, stale} = storeToRefs(skinStore)

const accountStore = useAccountStore()
const {accountConfig} = storeToRefs(accountStore)

const licensed = computed(() => (accountConfig.value?.accounts ?? []).filter(item => item.type === "microsoft"))

const activeUuid = ref<string | null>(null)

watch(licensed, list => {
  if (!activeUuid.value || !list.some(item => item.uuid === activeUuid.value)) {
    activeUuid.value = list[0]?.uuid ?? null
  }
}, {immediate: true})

const account = computed(() => licensed.value.find(item => item.uuid === activeUuid.value) ?? null)
const demo = computed(() => !account.value)

const accountItems = computed(() => licensed.value.map(item => ({
  label: item.name,
  value: item.uuid ?? item.name
})))

// превью

const POSES: { key: SkinPose, icon: string, label: string }[] = [
  {key: "stand", icon: "i-lucide-user-round", label: "Стоя"},
  {key: "walk", icon: "i-lucide-footprints", label: "Шаг"},
  {key: "run", icon: "i-lucide-wind", label: "Бег"}
]

const BACKGROUNDS = ["ink", "grid", "light"] as const
type Background = typeof BACKGROUNDS[number]

const pose = ref<SkinPose>("walk")
const spinning = ref(false)
const layers = ref(true)
const background = ref<Background>("grid")

const model = useTemplateRef("model")

const draftSkin = computed(() => skinStore.draftSkin)
const draftTexture = computed(() => skinStore.draftTexture)
const draftCape = computed(() => skinStore.draftCape)

function cycleBackground() {
  const next = BACKGROUNDS.indexOf(background.value) + 1
  background.value = BACKGROUNDS[next % BACKGROUNDS.length]!
}

const pickCape = (capeId: string | null) =>
    safeRun(() => skinStore.pickCape(capeId), {context: {action: "Плащ набора"}})

const setVariant = (variant: SkinVariant) =>
    safeRun(() => skinStore.setVariant(variant), {context: {action: "Модель рук"}})

// библиотека

const search = ref("")

const filtered = computed(() => {
  const needle = search.value.trim().toLowerCase()
  const skins = library.value.skins

  if (!needle) return skins

  return skins.filter(entry => entry.name.toLowerCase().includes(needle))
})

const importing = ref(false)
const dropping = ref(false)

const hovered = ref<string | null>(null)

async function importFile(path?: string) {
  if (importing.value) return

  importing.value = true

  const result = await attempt(() => skinStore.importFile(path), {context: {action: "Загрузка скина"}})

  importing.value = false

  if (result.ok && result.value) {
    toast.add({title: `«${result.value.name}» в библиотеке`, color: "success", icon: "i-lucide-image-plus"})
  }
}

let unlistenDrop: UnlistenFn | null = null

onMounted(async () => {
  unlistenDrop = await getCurrentWebview().onDragDropEvent(event => {
    if (event.payload.type === "over") {
      dropping.value = true
      return
    }

    if (event.payload.type !== "drop") {
      dropping.value = false
      return
    }

    dropping.value = false

    const png = event.payload.paths.find(path => path.toLowerCase().endsWith(".png"))

    if (!png) {
      toast.add({title: "Нужен png", description: "Скин Minecraft - это png 64x64", color: "error", icon: "i-lucide-file-x"})
      return
    }

    importFile(png)
  })
})

onBeforeUnmount(() => unlistenDrop?.())

// импорт по нику

const nickname = ref("")
const importingPlayer = ref(false)

async function importPlayer() {
  const name = nickname.value.trim()
  if (!name || importingPlayer.value) return

  importingPlayer.value = true

  const result = await attempt(() => skinStore.importPlayer(name), {context: {action: "Скин игрока"}})

  importingPlayer.value = false

  if (result.ok) {
    nickname.value = ""
    toast.add({title: `Скин ${result.value.name} загружен`, color: "success", icon: "i-lucide-user-round-search"})
  }
}

// копия набора

async function duplicate(entry: SkinEntry) {
  const result = await attempt(() => skinStore.duplicate(entry.id), {context: {action: "Копия набора"}})

  if (result.ok) {
    toast.add({
      title: `«${result.value.name}» создан`,
      color: "success",
      icon: "i-lucide-copy"
    })
  }
}

// переименование и удаление

const renameTarget = ref<SkinEntry | null>(null)
const renameValue = ref("")

function startRename(entry: SkinEntry) {
  renameTarget.value = entry
  renameValue.value = entry.name
}

async function commitRename() {
  const target = renameTarget.value
  if (!target) return

  renameTarget.value = null

  await safeRun(() => skinStore.rename(target.id, renameValue.value), {context: {action: "Переименование набора"}})
}

const removeTarget = ref<SkinEntry | null>(null)

async function confirmRemove() {
  const target = removeTarget.value
  if (!target) return

  removeTarget.value = null

  const result = await attempt(() => skinStore.remove(target.id), {context: {action: "Удаление набора"}})

  if (result.ok) {
    toast.add({title: `«${target.name}» удалён из библиотеки`, color: "success", icon: "i-lucide-trash-2"})
  }
}

// применение

const now = ref(Date.now())
let ticker: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  ticker = setInterval(() => now.value = Date.now(), 500)
})

onBeforeUnmount(() => {
  if (ticker) clearInterval(ticker)
})

const cooldown = computed(() => Math.max(0, Math.ceil((skinStore.cooldownUntil - now.value) / 1000)))

const canSave = computed(() => !demo.value && skinStore.dirty && !cooldown.value)

async function save() {
  if (demo.value) {
    toast.add({
      title: "Нужен аккаунт Microsoft",
      description: "Скин меняется только у лицензии",
      color: "error",
      icon: "i-lucide-lock"
    })
    return false
  }

  const result = await attempt(() => skinStore.save(), {context: {action: "Смена скина"}})

  if (!result.ok || !result.value) return false

  toast.add({
    title: `Внешний вид ${skinStore.name} обновлён`,
    description: "Отменить можно в течение минуты",
    color: "success",
    icon: "i-lucide-check",
    actions: [{
      label: "Отменить",
      color: "neutral",
      variant: "outline",
      onClick: () => undo()
    }]
  })

  return true
}

async function undo() {
  const result = await attempt(() => skinStore.undo(), {context: {action: "Отмена смены скина"}})

  if (result.ok && result.value) {
    toast.add({title: "Вернули как было", color: "success", icon: "i-lucide-undo-2"})
  }
}

const resetOpen = ref(false)

async function resetSkin() {
  resetOpen.value = false

  const result = await attempt(() => skinStore.resetSkin(), {context: {action: "Сброс скина"}})

  if (result.ok) toast.add({title: "Вернули стандартный скин", color: "success", icon: "i-lucide-rotate-ccw"})
}

const guard = useUnsavedChanges({
  dirty: () => skinStore.dirty,
  canSave,
  save,
  discard: () => skinStore.reset()
})

const VARIANTS: SkinVariant[] = ["CLASSIC", "SLIM"]

async function reload(uuid: string | null) {
  if (uuid) {
    await safeRun(() => skinStore.load(uuid), {context: {action: "Скины аккаунта"}})
    return
  }

  await safeRun(() => skinStore.loadLibrary(), {context: {action: "Библиотека скинов"}})
}

onMounted(() => reload(activeUuid.value))

watch(activeUuid, uuid => reload(uuid))
</script>

<template>
  <div class="min-h-full w-full px-8 pb-16 xl:px-14">
    <div class="grid gap-10 lg:grid-cols-[22rem_minmax(0,1fr)] lg:gap-12">
      <aside class="pt-10 lg:sticky lg:top-0 lg:self-start">
        <p class="font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">Внешний вид</p>
        <h1 class="mt-4 font-unbounded text-[clamp(26px,3vw,34px)] font-bold leading-[0.95] tracking-[-0.055em] text-fg">
          Скины<span class="text-acid">.</span>
        </h1>

        <div class="mt-6 flex items-center gap-3 border border-line bg-ink-800 px-4 py-3">
          <img
              v-if="account"
              :src="`https://assets.zaralx.ru/api/v1/minecraft/vanilla/player/face/${account.name}/full`"
              class="size-7 shrink-0"
              :alt="account.name"
              @error="fallbackFace"
          />
          <span v-else class="grid size-7 shrink-0 place-items-center border border-line text-fg-faint">
            <UIcon name="i-lucide-user-round" class="size-3.5"/>
          </span>

          <div class="min-w-0 flex-1">
            <USelect
                v-if="accountItems.length > 1"
                v-model="activeUuid"
                :items="accountItems"
                value-key="value"
                class="w-full"
            />
            <template v-else>
              <p class="truncate text-[13px] text-fg">{{ account?.name ?? 'Нет лицензии' }}</p>
              <p class="mt-0.5 font-mono text-[9px] uppercase tracking-[0.2em] text-fg-faint">
                {{ account ? 'Microsoft' : 'Только библиотека' }}
              </p>
            </template>
          </div>
        </div>

        <div
            v-if="demo"
            class="mt-3 flex items-start gap-2.5 border border-amber-400/30 bg-amber-400/[0.06] px-4 py-3"
        >
          <span class="mt-1 size-1.5 shrink-0 bg-amber-400 animate-blink"/>
          <p class="text-[11px] leading-relaxed text-amber-200/80">
            Скин меняется только у лицензии.
            <NuxtLink to="/settings" class="text-amber-300 underline underline-offset-2">Добавить Microsoft</NuxtLink>
            - библиотеку можно собирать и сейчас.
          </p>
        </div>

        <div
            v-else-if="stale"
            class="mt-3 flex items-start gap-2.5 border border-line bg-ink-800 px-4 py-3"
        >
          <UIcon name="i-lucide-cloud-off" class="mt-0.5 size-3.5 shrink-0 text-fg-faint"/>
          <p class="text-[11px] leading-relaxed text-fg-muted">
            Профиль Mojang не ответил - показываем сохранённое состояние.
          </p>
        </div>

        <div
            class="relative mt-5 h-[23rem] border border-line cut-16 transition-colors duration-500"
            :class="{
              'bg-ink-800': background === 'ink',
              'bg-ink-900': background === 'grid',
              'bg-fg-muted/90': background === 'light'
            }"
        >
          <div
              v-if="background === 'grid'"
              class="pointer-events-none absolute inset-0 opacity-[0.06]"
              style="background-image: linear-gradient(currentColor 1px, transparent 1px), linear-gradient(90deg, currentColor 1px, transparent 1px); background-size: 16px 16px"
              aria-hidden="true"
          />

          <div v-if="loading" class="grid h-full place-items-center">
            <span class="relative block h-px w-32 overflow-hidden bg-line">
              <span class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"/>
            </span>
          </div>

          <SkinModel
              v-else-if="draftTexture"
              ref="model"
              class="absolute inset-x-0 top-0 bottom-12"
              :skin="draftTexture"
              :cape="draftCape?.texture ?? null"
              :variant="draft.variant"
              :pose="pose"
              :spinning="spinning"
              :layers="layers"
          />

          <p
              v-else
              class="grid h-full place-items-center px-8 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
          >
            Набор не выбран
          </p>

          <div class="absolute inset-x-0 bottom-0 flex items-center justify-between border-t border-line bg-ink-900/70 px-2 py-2 backdrop-blur">
            <div class="flex items-center gap-0.5">
              <UButton
                  v-for="item in POSES"
                  :key="item.key"
                  color="neutral"
                  variant="ghost"
                  :icon="item.icon"
                  :title="item.label"
                  :aria-label="item.label"
                  class="size-8 justify-center transition-colors duration-300"
                  :class="pose === item.key ? 'text-acid hover:bg-transparent' : 'text-fg-faint hover:bg-transparent hover:text-fg'"
                  @click="pose = item.key"
              />

              <span class="mx-1 h-4 w-px bg-line"/>

              <UButton
                  color="neutral"
                  variant="ghost"
                  icon="i-lucide-rotate-3d"
                  title="Вращение"
                  aria-label="Вращение"
                  class="size-8 justify-center transition-colors duration-300"
                  :class="spinning ? 'text-acid hover:bg-transparent' : 'text-fg-faint hover:bg-transparent hover:text-fg'"
                  @click="spinning = !spinning"
              />

              <UButton
                  color="neutral"
                  variant="ghost"
                  icon="i-lucide-layers"
                  title="Второй слой"
                  aria-label="Второй слой"
                  class="size-8 justify-center transition-colors duration-300"
                  :class="layers ? 'text-acid hover:bg-transparent' : 'text-fg-faint hover:bg-transparent hover:text-fg'"
                  @click="layers = !layers"
              />
            </div>

            <div class="flex items-center gap-0.5">
              <UButton
                  color="neutral"
                  variant="ghost"
                  icon="i-lucide-sun-moon"
                  title="Фон превью"
                  aria-label="Фон превью"
                  class="size-8 justify-center text-fg-faint hover:bg-transparent hover:text-fg"
                  @click="cycleBackground"
              />

              <UButton
                  color="neutral"
                  variant="ghost"
                  icon="i-lucide-crosshair"
                  title="Сбросить поворот и зум"
                  aria-label="Сбросить поворот и зум"
                  class="size-8 justify-center text-fg-faint hover:bg-transparent hover:text-fg"
                  @click="model?.reset()"
              />
            </div>
          </div>
        </div>

        <div class="mt-4 flex border border-line">
          <button
              v-for="(variant, i) in VARIANTS"
              :key="variant"
              type="button"
              class="relative flex-1 px-4 py-2.5 font-mono text-[10px] uppercase tracking-[0.2em] transition-colors duration-300"
              :class="[
                i > 0 ? 'border-l border-line' : '',
                draft.variant === variant ? 'bg-ink-700 text-fg' : 'text-fg-faint hover:text-fg-muted'
              ]"
              :title="VARIANT_HINTS[variant]"
              @click="setVariant(variant)"
          >
            <span
                class="absolute inset-x-0 top-0 h-px origin-center bg-acid transition-transform duration-500 ease-deck"
                :class="draft.variant === variant ? 'scale-x-100' : 'scale-x-0'"
            />
            {{ VARIANT_LABELS[variant] }}
          </button>
        </div>

        <AppButton
            block
            class="mt-6 h-11 tracking-[0.2em]"
            icon="i-lucide-check"
            :loading="saving"
            :disabled="!canSave"
            @click="save"
        >
          <template v-if="saving">Применение</template>
          <template v-else-if="cooldown">Подождите {{ cooldown }}с</template>
          <template v-else>Применить</template>
        </AppButton>

        <div v-if="!demo" class="mt-4 flex items-center justify-between gap-3">
          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.18em]"
              icon="i-lucide-copy"
              :disabled="!draftSkin"
              @click="draftSkin && duplicate(draftSkin)"
          >
            Дублировать
          </AppButton>

          <AppButton
              tone="quiet"
              class="text-[10px] tracking-[0.18em] hover:text-red-400"
              icon="i-lucide-rotate-ccw"
              @click="resetOpen = true"
          >
            Стандартный
          </AppButton>
        </div>
        <div v-if="skinStore.dirty" class="mt-4 flex items-center justify-between gap-3">
          <AppButton tone="quiet" class="text-[10px] tracking-[0.18em] ml-auto mr-0" @click="skinStore.reset()">
            Вернуть
          </AppButton>
        </div>
      </aside>

      <div class="space-y-6 pt-10">
        <SettingsPanel
            index="01"
            title="Библиотека наборов"
            icon="i-lucide-shirt"
            class="animate-rise"
        >
          <div class="space-y-5">
            <div class="flex flex-wrap items-center justify-between gap-3">
<!--              <UInput v-model="search" placeholder="Поиск по названию" class="w-44">-->
<!--                <template #trailing>-->
<!--                  <UIcon name="i-lucide-search" class="size-3.5 text-fg-faint"/>-->
<!--                </template>-->
<!--              </UInput>-->

              <div class="flex items-center gap-2">
                <UInput
                    v-model="nickname"
                    placeholder="Ник игрока"
                    class="w-36"
                    @keyup.enter="importPlayer"
                >
                  <template #trailing>
                    <UIcon
                        :name="importingPlayer ? 'i-lucide-loader-circle' : 'i-lucide-user-round-search'"
                        class="size-3.5 text-fg-faint"
                        :class="importingPlayer ? 'animate-spin' : ''"
                    />
                  </template>
                </UInput>

                <AppButton
                    class="h-9 px-3.5 text-[10px] tracking-[0.18em]"
                    icon="i-lucide-upload"
                    :loading="importing"
                    @click="importFile()"
                >
                  Файл
                </AppButton>
              </div>
            </div>
            <div class="grid grid-cols-3 gap-2.5 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-6">
              <button
                  v-for="entry in filtered"
                  :key="entry.id"
                  type="button"
                  class="group/card relative flex aspect-[3/4] flex-col overflow-hidden border transition-colors duration-300"
                  :class="draft.skinId === entry.id
                    ? 'border-acid bg-ink-700'
                    : 'border-line hover:border-line-strong hover:bg-ink-700'"
                  @click="skinStore.pickSkin(entry.id)"
                  @mouseenter="hovered = entry.id"
                  @mouseleave="hovered = null"
              >
                <div class="relative min-h-0 flex-1">
                  <SkinModel
                      v-if="skinStore.textureOf(entry)"
                      class="absolute inset-0"
                      :skin="skinStore.textureOf(entry)!"
                      :cape="skinStore.capeById(entry.capeId ?? null)?.texture ?? null"
                      :variant="entry.variant"
                      :pose="hovered === entry.id ? 'walk' : 'stand'"
                      :angle="30"
                      :scale="0.9"
                      :interactive="false"
                  />
                  <span v-else class="absolute inset-0 m-auto h-16 w-8 bg-line/40"/>

                  <SkinCapeThumb
                      v-if="skinStore.capeById(entry.capeId ?? null)?.texture"
                      :cape="skinStore.capeById(entry.capeId ?? null)!.texture!"
                      :scale="1.5"
                      class="absolute bottom-1 right-1 border border-line/60"
                  />
                </div>

                <div class="w-full border-t border-line px-2 py-1.5 text-left">
                  <p class="truncate text-[10px] leading-tight text-fg">{{ entry.name }}</p>
                  <p class="mt-0.5 truncate font-mono text-[8px] uppercase tracking-[0.14em] text-fg-faint">
                    {{ VARIANT_LABELS[entry.variant] }} · {{ SOURCE_LABELS[entry.source] }}
                  </p>
                </div>

                <span
                    v-if="skinStore.applied.skinId === entry.id"
                    class="absolute left-0 top-0 bg-acid px-1.5 py-0.5 font-mono text-[7px] uppercase tracking-[0.18em] text-on-acid"
                >
                  Активен
                </span>

                <div class="absolute right-1 top-1 hidden gap-0.5 group-hover/card:flex">
                  <span
                      class="grid size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 hover:border-acid/50 hover:text-acid"
                      title="Дублировать с выбранным плащом"
                      @click.stop="duplicate(entry)"
                  >
                    <UIcon name="i-lucide-copy" class="size-2.5"/>
                  </span>

                  <span
                      class="grid size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 hover:border-acid/50 hover:text-acid"
                      title="Переименовать"
                      @click.stop="startRename(entry)"
                  >
                    <UIcon name="i-lucide-pencil" class="size-2.5"/>
                  </span>

                  <span
                      class="grid size-5 place-items-center border border-line bg-ink-800 text-fg-faint transition-colors duration-300 hover:border-red-400/50 hover:text-red-400"
                      title="Удалить"
                      @click.stop="removeTarget = entry"
                  >
                    <UIcon name="i-lucide-trash-2" class="size-2.5"/>
                  </span>
                </div>
              </button>

              <button
                  type="button"
                  class="group/drop flex aspect-[3/4] flex-col items-center justify-center gap-2 border border-dashed transition-colors duration-300"
                  :class="dropping ? 'border-acid bg-acid/[0.06]' : 'border-line hover:border-line-strong hover:bg-ink-700'"
                  @click="importFile()"
              >
                <UIcon
                    name="i-lucide-image-plus"
                    class="size-4 text-fg-faint transition-colors duration-300 group-hover/drop:text-acid"
                />
                <span class="px-2 text-center font-mono text-[8px] uppercase leading-relaxed tracking-[0.16em] text-fg-faint">
                  Перетащи<br>png 64x64
                </span>
              </button>
            </div>

            <p
                v-if="!filtered.length && !loading"
                class="border border-dashed border-line py-10 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint"
            >
              {{ search ? 'Ничего не найдено' : 'Библиотека пуста' }}
            </p>
          </div>
        </SettingsPanel>

        <SettingsPanel
            index="02"
            :title="draftSkin ? `Плащ набора «${draftSkin.name}»` : 'Плащ набора'"
            icon="i-lucide-flag"
            class="animate-rise [animation-delay:80ms]"
        >
          <div v-if="!draftSkin" class="border border-dashed border-line py-8 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            Сначала выбери набор
          </div>

          <div v-else-if="capes.length" class="space-y-4">
            <div class="flex flex-wrap gap-3">
              <button
                  type="button"
                  class="flex h-[6.5rem] w-[4.5rem] flex-col items-center justify-center gap-2 border transition-colors duration-300"
                  :class="draft.capeId === null
                    ? 'border-acid bg-ink-700'
                    : 'border-line hover:border-line-strong hover:bg-ink-700'"
                  @click="pickCape(null)"
              >
                <UIcon name="i-lucide-ban" class="size-4 text-fg-faint"/>
                <span class="font-mono text-[8px] uppercase tracking-[0.18em] text-fg-faint">Без плаща</span>
              </button>

              <button
                  v-for="cape in capes"
                  :key="cape.id"
                  type="button"
                  class="group/cape flex h-26 w-18 flex-col items-center justify-center gap-2 border transition-colors duration-300 cursor-pointer"
                  :class="draft.capeId === cape.id
                    ? 'border-acid bg-ink-700'
                    : 'border-line hover:border-line-strong hover:bg-ink-700'"
                  :title="cape.alias"
                  @click="pickCape(cape.id)"
              >
                <SkinCapeThumb
                    v-if="cape.texture"
                    :cape="cape.texture"
                    :scale="5"
                    class="transition-transform duration-500 ease-deck"
                />
              </button>
            </div>
          </div>

          <p v-else class="border border-dashed border-line py-8 text-center font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
            {{ demo ? 'Нужен аккаунт Microsoft' : 'Плащей на аккаунте нет' }}
          </p>
        </SettingsPanel>
      </div>
    </div>

    <UModal
        :open="!!renameTarget"
        title="Название набора"
        @update:open="value => { if (!value) renameTarget = null }"
    >
      <template #body>
        <UInput v-model="renameValue" class="w-full" autofocus @keyup.enter="commitRename"/>
      </template>

      <template #footer>
        <div class="flex w-full items-center justify-end gap-3">
          <AppButton tone="quiet" class="text-[10px] tracking-[0.18em]" @click="renameTarget = null">
            Отмена
          </AppButton>
          <AppButton class="h-8 text-[10px] tracking-[0.18em]" icon="i-lucide-check" @click="commitRename">
            Сохранить
          </AppButton>
        </div>
      </template>
    </UModal>

    <UModal
        :open="!!removeTarget"
        title="Удаление набора"
        @update:open="value => { if (!value) removeTarget = null }"
    >
      <template #body>
        <p class="text-[12px] leading-relaxed text-fg-muted">
          «{{ removeTarget?.name }}» пропадёт из библиотеки лаунчера. Копии с тем же скином останутся,
          а на аккаунте Mojang это никак не отразится.
        </p>
      </template>

      <template #footer>
        <div class="flex w-full items-center justify-end gap-3">
          <AppButton tone="quiet" class="text-[10px] tracking-[0.18em]" @click="removeTarget = null">
            Отмена
          </AppButton>
          <AppButton
              class="h-8 text-[10px] tracking-[0.18em] hover:border-red-500 hover:text-white hover:before:bg-red-500"
              icon="i-lucide-trash-2"
              @click="confirmRemove"
          >
            Удалить
          </AppButton>
        </div>
      </template>
    </UModal>

    <UModal
        v-model:open="resetOpen"
        title="Стандартный скин"
    >
      <template #body>
        <p class="text-[12px] leading-relaxed text-fg-muted">
          Аккаунт вернётся к скину Steve или Alex. Библиотеку это не тронет - набор можно поставить обратно.
        </p>
      </template>

      <template #footer>
        <div class="flex w-full items-center justify-end gap-3">
          <AppButton tone="quiet" class="text-[10px] tracking-[0.18em]" @click="resetOpen = false">
            Отмена
          </AppButton>
          <AppButton
              class="h-8 text-[10px] tracking-[0.18em]"
              icon="i-lucide-rotate-ccw"
              :loading="saving"
              @click="resetSkin"
          >
            Сбросить
          </AppButton>
        </div>
      </template>
    </UModal>

    <UnsavedChangesModal
        :guard="guard"
        description="Набор выбран, но ещё не применён к аккаунту."
        blocked="Применить сейчас нельзя: нет лицензии или не вышла пауза Mojang."
        discard-label="Не применять"
    />
  </div>
</template>
