<script setup lang="ts">
import type {Instance} from "~/types/instance"
import type {CastPackUpdate} from "~/types/castpack"
import {useCastPackStore} from "~/stores/castpack"

const props = defineProps<{ instance: Instance }>()

const instanceStore = useInstanceStore()
const castpackStore = useCastPackStore()
const toast = useToast()

const checking = ref(false)
const update = ref<CastPackUpdate | null>(null)

const source = computed(() => props.instance.castpack)
const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))

const autoupdate = computed({
  get: () => source.value?.autoupdate ?? false,
  set: (enabled: boolean) => toggle(enabled)
})

const blocked = computed(() => {
  if (running.value) return "Сборка запущена - сначала закройте игру"
  if (installing.value) return "Дождитесь окончания текущей установки"
  return null
})

const facts = computed(() => [
  {label: "Сборка", value: source.value?.catalogId ?? "-"},
  {label: "Установленная версия", value: source.value?.version || "ещё не установлена"},
  {label: "Автообновление", value: source.value?.autoupdate ? "включено" : "выключено"},
  {label: "Манифест", value: source.value?.manifestUrl ?? "-"}
])

async function check() {
  if (checking.value) return

  checking.value = true

  const result = await attempt(() => castpackStore.checkUpdate(props.instance.id), {
    code: "NETWORK",
    context: {instanceId: props.instance.id, action: "Проверка обновления сборки"}
  })

  checking.value = false

  if (result.ok) update.value = result.value
}

onMounted(check)

async function toggle(enabled: boolean) {
  const result = await attempt(() => castpackStore.setAutoupdate(props.instance.id, enabled), {
    context: {instanceId: props.instance.id, action: "Переключение автообновления"}
  })

  if (!result.ok) return

  toast.add({
    title: enabled ? "Автообновление включено" : "Автообновление выключено",
    description: enabled
        ? "Перед запуском лаунчер будет догонять новые версии сборки"
        : "Обновлять сборку придётся кнопкой вручную",
    color: "success",
    icon: "i-lucide-refresh-cw"
  })
}

async function reinstall() {
  if (blocked.value) return

  const started = await attempt(() => instanceStore.installInstance(props.instance.id), {
    code: "NETWORK",
    context: {instanceId: props.instance.id, action: "Переустановка сборки"}
  })

  if (!started.ok) return

  toast.add({
    title: "Файлы сборки проверяются",
    description: "Недостающее докачается, лишнее уберётся",
    color: "success",
    icon: "i-lucide-refresh-cw"
  })
}

const openSite = (url: string) => safeRun(() => call("open_url", {url}))
</script>

<template>
  <div class="space-y-6">
    <SettingsPanel index="01" title="Сборка CastPack" icon="i-lucide-layers">
      <div v-if="!source" class="text-[12px] leading-relaxed text-fg-muted">
        Эта сборка не из каталога CastPack.
      </div>

      <div v-else class="space-y-7">
        <div
            v-if="update?.available"
            class="flex items-start justify-between gap-4 border border-acid/30 bg-acid/[0.04] px-4 py-3"
        >
          <div class="min-w-0">
            <p class="text-[12px] leading-relaxed text-fg-muted">
              Доступна версия <span class="text-fg">{{ update.version }}</span>.
              Она установится сама при следующем запуске.
            </p>
            <p v-if="update.changelog" class="mt-2 whitespace-pre-line text-[12px] leading-relaxed text-fg-muted">
              {{ update.changelog }}
            </p>
          </div>

          <AppButton
              tone="quiet"
              class="shrink-0 text-[10px] tracking-[0.18em]"
              icon="i-lucide-arrow-down-to-line"
              :disabled="!!blocked"
              @click="reinstall"
          >
            Обновить
          </AppButton>
        </div>

        <p
            v-else-if="update?.error"
            class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted"
        >
          <UIcon name="i-lucide-wifi-off" class="mt-0.5 size-3.5 shrink-0 text-amber-400"/>
          Проверить обновление не вышло: {{ update.error }}. Играть это не мешает.
        </p>

        <p v-else-if="update" class="flex items-start gap-2.5 text-[12px] leading-relaxed text-fg-muted">
          <UIcon name="i-lucide-check" class="mt-0.5 size-3.5 shrink-0 text-acid"/>
          Установлена последняя версия сборки.
        </p>

        <SettingsField
            label="Автообновление"
            hint="Перед запуском лаунчер сверяется с манифестом и докачивает новую версию"
        >
          <USwitch v-model="autoupdate"/>
        </SettingsField>

        <div class="flex items-center justify-between gap-6 border-t border-line pt-6">
          <p class="min-w-0 text-[12px] leading-relaxed text-fg-muted">
            Моды и конфиги сборки лаунчер держит сам: изменённые файлы перезапишутся, а те, что
            исчезли из новой версии, удалятся. Миры, скриншоты и настройки игры останутся на месте.
          </p>

          <div class="flex shrink-0 gap-2">
            <AppButton
                tone="quiet"
                class="h-9 text-[10px] tracking-[0.18em]"
                icon="i-lucide-rotate-cw"
                :loading="checking"
                @click="check"
            >
              Проверить
            </AppButton>

            <AppButton
                class="h-9 text-[10px] tracking-[0.18em]"
                icon="i-lucide-wrench"
                :disabled="!!blocked"
                @click="reinstall"
            >
              Починить файлы
            </AppButton>
          </div>
        </div>

        <p v-if="blocked" class="font-mono text-[10px] uppercase tracking-[0.2em] text-fg-faint">
          {{ blocked }}
        </p>
      </div>
    </SettingsPanel>

    <SettingsPanel v-if="source?.changelog" index="02" title="Что изменилось" icon="i-lucide-scroll-text">
      <p class="whitespace-pre-line text-[12px] leading-relaxed text-fg-muted">{{ source.changelog }}</p>
    </SettingsPanel>

    <SettingsPanel :index="source?.changelog ? '03' : '02'" title="Источник" icon="i-lucide-link">
      <dl class="grid gap-x-6 gap-y-4 sm:grid-cols-2">
        <div v-for="fact in facts" :key="fact.label" class="min-w-0">
          <dt class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">{{ fact.label }}</dt>
          <dd class="mt-1.5 truncate font-mono text-[12px] text-fg-muted" :title="fact.value">{{ fact.value }}</dd>
        </div>
      </dl>

      <AppButton
          v-if="source?.manifestUrl"
          tone="quiet"
          class="mt-5 text-[10px] tracking-[0.16em]"
          icon="i-lucide-external-link"
          @click="openSite(source.manifestUrl)"
      >
        Открыть манифест
      </AppButton>
    </SettingsPanel>
  </div>
</template>
