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

async function saveConfig() {
  if (!config.value || saving.value) return;
  saving.value = true
  try {
    await store.updateConfig(config.value)
    toast.add({
      title: 'Настройки сохранены',
      color: 'success',
      icon: 'i-lucide-save'
    })
  } catch (e) {
    toast.add({
      title: 'Произошла ошибка',
      description: 'Не получилось сохранить настройки',
      color: 'error',
      icon: 'i-lucide-save'
    })
  } finally {
    saving.value = false
  }
}
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
            :disabled="!config"
            @click="saveConfig"
        >
          {{ saving ? 'Сохранение' : 'Сохранить' }}
        </AppButton>
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
  </div>
</template>
