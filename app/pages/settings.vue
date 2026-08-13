<script setup lang="ts">
import {useAppStore} from "~/stores/app";
import {storeToRefs} from "pinia";

definePageMeta({
  layout: "main"
});

const store = useAppStore();
const {config} = storeToRefs(store);

const toast = useToast()
const saving = ref(false)

const saved = ref<string | null>(null)

watch(config, value => {
  if (value && saved.value === null) saved.value = JSON.stringify(value)
}, {immediate: true})

const dirty = computed(() => {
  if (!config.value || saved.value === null) return false
  return JSON.stringify(config.value) !== saved.value
})

async function saveConfig() {
  if (!config.value || saving.value) return false;
  saving.value = true
  try {
    await store.updateConfig(config.value)
    saved.value = JSON.stringify(config.value)
    toast.add({
      title: 'Настройки сохранены',
      color: 'success',
      icon: 'i-lucide-save'
    })
    return true
  } catch (e) {
    toast.add({
      title: 'Произошла ошибка',
      description: 'Не получилось сохранить настройки',
      color: 'error',
      icon: 'i-lucide-save'
    })
    return false
  } finally {
    saving.value = false
  }
}

function reset() {
  if (saved.value !== null) config.value = JSON.parse(saved.value)
}

const guard = useUnsavedChanges({
  dirty,
  save: saveConfig,
  discard: reset
})
</script>

<template>
  <div class="min-h-full w-full px-8 pb-16 xl:px-14">
    <div class="grid gap-10 lg:grid-cols-[15rem_minmax(0,1fr)] lg:gap-14">
      <aside class="lg:sticky pt-10 lg:top-0 lg:self-start">
        <p class="font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">Конфигурация</p>
        <h1 class="mt-4 font-unbounded text-[clamp(26px,3vw,34px)] font-bold leading-[0.95] tracking-[-0.055em] text-fg">
          Настройки<span class="text-acid">.</span>
        </h1>
        <p class="mt-5 text-[12px] leading-relaxed text-fg-muted">
          Не забудь нажать кнопочку ниже для сохранения
        </p>

        <AppButton
            block
            class="mt-8 h-11 tracking-[0.2em]"
            icon="i-lucide-save"
            :loading="saving"
            :disabled="!config || !dirty"
            @click="saveConfig"
        >
          {{ saving ? 'Сохранение' : 'Сохранить' }}
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
      </aside>

      <div v-if="config" class="space-y-6 pt-10">
        <SettingsLauncher v-model="config" class="animate-rise"/>
        <SettingsAccounts class="animate-rise [animation-delay:80ms]"/>
        <SettingsJava v-model="config" class="animate-rise [animation-delay:160ms]"/>
        <SettingsImport class="animate-rise [animation-delay:240ms]"/>

        <NuxtLink to="/castpack-editor" class="block">
          <AppButton block class="h-11 tracking-[0.2em] opacity-0" icon="i-lucide-braces">
            Редактор сборок CastPack
          </AppButton>
        </NuxtLink>
      </div>

      <div v-else class="flex items-center gap-3 py-14">
        <span class="size-1.5 bg-fg-faint animate-blink"/>
        <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Конфигурация не загружена</p>
      </div>
    </div>

    <UnsavedChangesModal
        :guard="guard"
        description="Настройки лаунчера изменены, но не записаны на диск. Уйти без сохранения - значит вернуть их как было."
        discard-label="Вернуть как было"
    />
  </div>
</template>
